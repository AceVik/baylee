//! `cards/creatures/artifacts/mv_5/clockwork_avian.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Clockwork Avian — flying, "This creature enters with four +1/+0 counters
/// on it" (a replacement on the way in, CR 614.1c), and "At end of combat,
/// if this creature attacked or blocked this combat, remove a +1/+0 counter
/// from it" (CR 511.2). The seated Avian is 4/4 by counters over a printed
/// 0/4; a combat it sat out costs it nothing, and the combat it attacks
/// loses exactly one counter as the end-of-combat step begins — after its
/// four damage has been dealt.
#[test]
fn clockwork_avian_loses_one_counter_only_for_a_combat_it_joined() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[clockwork_avian()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let avian = on_battlefield(&engine, p0, clockwork_avian()).expect("seated");
    assert_eq!(plus_one_zero(&engine, avian), 4, "entered with four");
    assert_eq!(
        pt(&engine, avian),
        (4, 4),
        "printed 0/4 plus four +1/+0 counters"
    );
    assert!(
        keywords(&engine, avian).contains(KeywordSet::FLYING),
        "flying is the card's first line"
    );

    // A combat it does not join: the trigger's condition is false.
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(
        plus_one_zero(&engine, avian),
        4,
        "not attacking or blocking removes nothing"
    );

    // The next turn it attacks; the counter comes off at end of combat.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(avian, Defender::Player(p1))],
            },
        )
        .expect("the Avian attacks");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("no blocks");
    assert_eq!(
        plus_one_zero(&engine, avian),
        4,
        "still four while the damage step is ahead"
    );
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(
        engine.state().players[1].life,
        16,
        "the four counters' power"
    );
    assert_eq!(
        plus_one_zero(&engine, avian),
        3,
        "one removed at end of combat"
    );
    assert_eq!(pt(&engine, avian), (3, 4));
}

/// The capped recharge: "{X}, {T}: Put up to X +1/+0 counters on this
/// creature. This ability can't cause the total number of +1/+0 counters on
/// this creature to be greater than four. Activate only during your upkeep"
/// (CR 602.5). At the maximum, X is announced and paid, the effect adds
/// nothing, and no "up to X" question is ever asked — the ruling's "counters
/// over the maximum are simply not added".
#[test]
fn clockwork_avian_recharge_at_four_counters_adds_none() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[clockwork_avian(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let avian = on_battlefield(&engine, p0, clockwork_avian()).expect("seated");

    // A main phase with mana floating: the offer's window is the upkeep.
    let first = all_on_battlefield(&engine, p0, forest())[0];
    tap_mana_where(&mut engine, p0, |id| id == first);
    assert!(
        !priority_offer(&engine).abilities.contains(&(avian, 1)),
        "the main phase is not the upkeep"
    );

    // Next upkeep at the maximum of four: X=1 buys no room.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0
            && e.state().turn.step == Step::Upkeep
            && e.state().turn.number > 1
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert_eq!(plus_one_zero(&engine, avian), 4);
    let one = all_on_battlefield(&engine, p0, forest())[0];
    tap_mana_where(&mut engine, p0, |id| id == one);
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);

    activate(&mut engine, p0, clockwork_avian(), 1);
    let Pending::ChooseNumber {
        player, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("{{X}} announces X first: {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    assert!(min <= 1 && max >= 1, "one floating pays X=1: {min}..{max}");
    engine.apply(p0, PlayerAction::ChooseNumber(1)).unwrap();
    // Room is zero, so the "up to X" question is never asked: the ability
    // resolves straight through. `pass_until` has no arm for a number
    // question, so one arriving here would panic, which is the point.
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(plus_one_zero(&engine, avian), 4, "four is the cap");
    assert!(is_tapped(&engine, avian), "{{T}} was paid");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and X was paid even though nothing was added"
    );
}

/// The recharge below the cap: after the attack costs it one counter, the
/// upkeep's "{X}, {T}" adds up to X but no more than the room left under
/// four. X=2 with one free slot asks for at most one (CR 608.2d — "up to"
/// includes zero and the cap bounds the maximum).
#[test]
fn clockwork_avian_recharge_refills_only_up_to_four() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[clockwork_avian(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let avian = on_battlefield(&engine, p0, clockwork_avian()).expect("seated");

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(avian, Defender::Player(p1))],
            },
        )
        .expect("the Avian attacks");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("no blocks");
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(plus_one_zero(&engine, avian), 3, "the attack cost it one");

    pass_until(&mut engine, |e| {
        e.state().turn.active == p0
            && e.state().turn.step == Step::Upkeep
            && e.state().turn.number > 1
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    let forests = all_on_battlefield(&engine, p0, forest());
    tap_mana_where(&mut engine, p0, |id| forests[..2].contains(&id));
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);

    activate(&mut engine, p0, clockwork_avian(), 1);
    let Pending::ChooseNumber { player, .. } = engine.pending().clone() else {
        panic!("{{X}} announces X first: {:?}", engine.pending())
    };
    engine.apply(player, PlayerAction::ChooseNumber(2)).unwrap();
    // The announcement is paid, the ability goes on the stack, and the
    // "up to X" question arrives as it resolves.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    let Pending::ChooseNumber {
        player, min, max, ..
    } = engine.pending().clone()
    else {
        unreachable!("the pass waited for exactly this")
    };
    assert_eq!(
        (player, min, max),
        (p0, 0, 1),
        "one free slot under the cap"
    );
    engine.apply(player, PlayerAction::ChooseNumber(1)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        plus_one_zero(&engine, avian),
        4,
        "refilled to the cap and no further"
    );
    assert_eq!(pt(&engine, avian), (4, 4));
    assert!(is_tapped(&engine, avian));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}
