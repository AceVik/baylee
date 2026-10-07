//! `cards/creatures/mv_5/thundering_giant.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Thundering Giant` is a 4/3 creature costing `{3}{R}{R}` under `Coverage::Implemented`.
/// It prints the haste keyword.
/// When cast from hand off five Mountains, it resolves and can attack immediately
/// during the combat phase of the same turn, bypassing summoning sickness.
#[test]
fn thundering_giant_attacks_on_turn_it_enters_due_to_haste() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[thundering_giant()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, thundering_giant());
    pass_until(&mut engine, stack_is_empty);

    let giant = on_battlefield(&engine, p0, thundering_giant())
        .expect("Thundering Giant resolved and is on the battlefield");
    assert_eq!(pt(&engine, giant), (4, 3), "printed power/toughness is 4/3");
    assert!(
        keywords(&engine, giant).contains(KeywordSet::HASTE),
        "Thundering Giant has haste"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseAttackers prompt, got {:?}",
            engine.pending()
        );
    };
    assert!(
        attackers.contains(&giant),
        "haste allows creature to attack on arrival turn: {attackers:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(giant, Defender::Player(p1))],
            },
        )
        .unwrap();

    assert!(
        is_tapped(&engine, giant),
        "creature without vigilance taps when declared as an attacker"
    );
}

/// "Questing Beast can't be blocked by creatures with power 2 or less." The
/// 2/2 is offered no block on it and the 4/3 is; vigilance, deathtouch and
/// haste are printed.
#[test]
fn questing_beast_cannot_be_blocked_by_power_two_or_less() {
    let p1 = PlayerId::new(1);
    let (mut engine, beast) = questing_beast_attacks(&[steadfast_guard(), thundering_giant()]);
    let kw = keywords(&engine, beast);
    assert!(
        kw.contains(
            KeywordSet::VIGILANCE
                .union(KeywordSet::DEATHTOUCH)
                .union(KeywordSet::HASTE)
        )
    );
    assert!(
        !is_tapped(&engine, beast),
        "vigilance: attacking did not tap it"
    );
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!()
    };
    let may_block = |card: CardIndex| {
        let id = on_battlefield(&engine, p1, card).unwrap();
        blockers
            .iter()
            .any(|o| o.blocker == id && o.attackers.contains(&beast))
    };
    assert!(!may_block(steadfast_guard()), "a 2/2 has power 2 or less");
    assert!(may_block(thundering_giant()), "a 4/3 does not");
}

/// "Whenever a creature an opponent controls with a -1/-1 counter on it
/// dies, you may put that card onto the battlefield under your control. Do
/// this only once each turn." The Guard dies wearing The Reaper's counter
/// and comes over; the Giant dies wearing one the same turn, and the
/// ability asks nothing.
#[test]
fn the_reaper_takes_one_marked_creature_each_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = the_reaper_marks_two();
    let guard = on_battlefield(&engine, p1, steadfast_guard()).unwrap();
    let giant = on_battlefield(&engine, p1, thundering_giant()).unwrap();

    reaper_sees_die(&mut engine, guard);
    assert!(asked_may(&engine), "got {:?}", engine.pending());
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    let guard = on_battlefield(&engine, p0, steadfast_guard()).expect("under your control");
    assert_eq!(engine.state().object(guard).unwrap().owner, p1);
    assert_eq!(
        counters_on(&engine, guard, CounterKind::M1M1),
        0,
        "a new object (CR 400.7)"
    );

    reaper_sees_die(&mut engine, giant);
    assert!(!asked_may(&engine), "only once each turn");
    assert!(stack_is_empty(&engine));
    assert!(in_graveyard(&engine, p1, thundering_giant()).is_some());
}

/// The Elves die wearing no counter and nothing is asked, with the turn's
/// go still unused. Then a no keeps that go: the Guard is left in the
/// graveyard, and the Giant dying next is asked about and comes over.
#[test]
fn the_reaper_declined_keeps_its_one_go() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = the_reaper_marks_two();
    let guard = on_battlefield(&engine, p1, steadfast_guard()).unwrap();
    let giant = on_battlefield(&engine, p1, thundering_giant()).unwrap();
    let elves = on_battlefield(&engine, p1, llanowar_elves()).unwrap();

    reaper_sees_die(&mut engine, elves);
    assert!(!asked_may(&engine), "no -1/-1 counter on it");
    assert!(in_graveyard(&engine, p1, llanowar_elves()).is_some());

    reaper_sees_die(&mut engine, guard);
    assert!(asked_may(&engine));
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p1, steadfast_guard()).is_some());

    reaper_sees_die(&mut engine, giant);
    assert!(asked_may(&engine), "the go was not used");
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, thundering_giant()).is_some());
}

/// CR 608.2d: "put that card onto the battlefield" is impossible once the
/// card has left the graveyard, so nothing is asked and the turn's go is
/// kept. The Guard's card is exiled with the trigger waiting on the stack;
/// the Giant dying next is asked about.
#[test]
fn the_reaper_asks_nothing_for_a_card_that_left_the_graveyard() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = the_reaper_marks_two();
    let guard = on_battlefield(&engine, p1, steadfast_guard()).unwrap();
    let giant = on_battlefield(&engine, p1, thundering_giant()).unwrap();

    {
        let state = engine
            .dev_state_mut(p0)
            .expect("the harness may set boards up");
        crate::sba::destroy(state, guard);
    }
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    engine.apply(player, PlayerAction::PassPriority).unwrap();
    assert!(!stack_is_empty(&engine), "the trigger waits on the stack");
    let card = in_graveyard(&engine, p1, steadfast_guard()).unwrap();
    {
        let state = engine
            .dev_state_mut(p0)
            .expect("the harness may set boards up");
        state
            .move_object(
                card,
                ZoneLocation::Exile(p1),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
    }
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. }) || stack_is_empty(e)
    });
    assert!(
        !asked_may(&engine),
        "no card left to put onto the battlefield"
    );

    reaper_sees_die(&mut engine, giant);
    assert!(asked_may(&engine), "the go was kept");
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, thundering_giant()).is_some());
}

/// "When The Balrog of Moria dies, you may exile it. When you do, for each
/// opponent, exile up to one target creature that player controls." Two
/// opponents: the targets are asked after the exile, one opponent at a
/// time, each menu holding only that player's creatures, and one creature
/// of each is exiled.
#[test]
fn the_balrog_of_moria_exiles_itself_and_a_creature_of_each_opponent() {
    let p0 = PlayerId::new(0);
    let (p1, p2) = (PlayerId::new(1), PlayerId::new(2));
    let (mut engine, _) =
        balrog_dies(&[&[thundering_giant(), llanowar_elves()], &[llanowar_elves()]]);
    let giant = on_battlefield(&engine, p1, thundering_giant()).unwrap();
    let elves_1 = on_battlefield(&engine, p1, llanowar_elves()).unwrap();
    let elves_2 = on_battlefield(&engine, p2, llanowar_elves()).unwrap();
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    assert!(
        balrog_in_exile(&engine),
        "exiled before any target is named"
    );
    pass_until(&mut engine, |e| {
        !matches!(e.pending(), Pending::Priority { .. })
    });
    for (options, pick) in [(vec![giant, elves_1], giant), (vec![elves_2], elves_2)] {
        let Pending::ChooseTargets {
            player,
            options: offered,
            min: 0,
            max: 1,
            ..
        } = engine.pending().clone()
        else {
            panic!("one opponent's creatures, got {:?}", engine.pending())
        };
        assert_eq!(player, p0);
        let mut offered = offered;
        offered.sort_unstable();
        let mut options = options;
        options.sort_unstable();
        assert_eq!(offered, options);
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![pick],
                    players: vec![],
                },
            )
            .unwrap();
    }
    // On the stack with both, and what they were chosen against admits two.
    let top = *engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .last()
        .unwrap();
    let reflexive = engine.state().object(top).unwrap();
    assert_eq!(reflexive.targets.len(), 2);
    assert!(
        reflexive
            .target_req
            .is_some_and(|req| usize::from(req.max) >= reflexive.targets.len()),
        "{:?}",
        reflexive.target_req
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p1, thundering_giant()).is_none());
    assert!(on_battlefield(&engine, p2, llanowar_elves()).is_none());
    assert_eq!(on_battlefield(&engine, p1, llanowar_elves()), Some(elves_1));
}

/// One target is dealt all 4, and nothing is asked: there is nothing to
/// divide.
#[test]
fn fury_with_one_target_deals_it_all_four() {
    let p1 = PlayerId::new(1);
    let mut engine = fury_enters(&[thundering_giant()], false);
    let giant = on_battlefield(&engine, p1, thundering_giant()).unwrap();
    fury_aims(&mut engine, vec![giant]);
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "got {:?}",
        engine.pending()
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p1, thundering_giant()).is_some());
}

/// A target gone before the trigger resolves is not dealt its share, and
/// nobody else is either: the division was fixed as the trigger was put on
/// the stack (CR 601.2d), and an illegal target is simply not affected
/// (CR 608.2b). The Elves were given 3 and the Giant 1; with the Elves gone
/// the Giant is dealt its own 1, neither the Elves' 3 nor all 4.
#[test]
fn fury_loses_the_share_of_a_target_that_is_gone() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = fury_enters(&[llanowar_elves(), thundering_giant()], false);
    let elves = on_battlefield(&engine, p1, llanowar_elves()).unwrap();
    let giant = on_battlefield(&engine, p1, thundering_giant()).unwrap();
    fury_aims(&mut engine, vec![elves, giant]);
    assert_eq!(fury_share(&engine, elves, 0, 2, 4), (1, 3));
    engine.apply(p0, PlayerAction::ChooseNumber(3)).unwrap();
    engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up")
        .move_object(
            elves,
            ZoneLocation::Graveyard(p1),
            ZonePosition::Top,
            crate::event::Cause::DevCommand,
        )
        .expect("the Elves leave");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(marked(&engine, giant), 1, "the Giant's own share");
    assert!(on_battlefield(&engine, p1, thundering_giant()).is_some());
}
