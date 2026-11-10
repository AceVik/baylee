//! Soul Charmer — {2}{W} — Creature — Human Rebel
//! Oracle: Whenever this creature deals combat damage to a creature, you gain 2 life unless that creature's controller pays {2}.

use super::*;

#[test]
fn soul_charmer_gains_two_life_when_damaged_creatures_controller_declines_to_pay() {
    let soul_charmer = card_index("de56a8d9-faeb-4e73-bdc1-cc81a6b6d05c");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1, plains())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[soul_charmer])
        .battlefield(1, &[grizzly_bears(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, soul_charmer);
    pass_until(&mut engine, stack_is_empty);
    assert!(walk_to_own_main(&mut engine, p1));
    assert!(walk_to_own_main(&mut engine, p0));

    let charmer_id = on_battlefield(&engine, p0, soul_charmer).expect("Soul Charmer on board");
    let bears_id = on_battlefield(&engine, p1, grizzly_bears()).expect("Bears on board");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(charmer_id, Defender::Player(p1))],
            },
        )
        .expect("declare Soul Charmer as attacker");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bears_id, charmer_id)],
            },
        )
        .expect("declare Bears as blocker");

    pass_until(&mut engine, |e| {
        !stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    tap_all_mana(&mut engine, p1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine
        .apply(p1, PlayerAction::YesNo(false))
        .expect("decline to pay {2}");

    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(engine.state().players[0].life, 22);
}

#[test]
fn soul_charmer_gains_no_life_when_damaged_creatures_controller_pays_two() {
    let soul_charmer = card_index("de56a8d9-faeb-4e73-bdc1-cc81a6b6d05c");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(2, plains())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[soul_charmer])
        .battlefield(1, &[grizzly_bears(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, soul_charmer);
    pass_until(&mut engine, stack_is_empty);
    assert!(walk_to_own_main(&mut engine, p1));
    assert!(walk_to_own_main(&mut engine, p0));

    let charmer_id = on_battlefield(&engine, p0, soul_charmer).expect("Soul Charmer on board");
    let bears_id = on_battlefield(&engine, p1, grizzly_bears()).expect("Bears on board");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(charmer_id, Defender::Player(p1))],
            },
        )
        .expect("declare Soul Charmer as attacker");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bears_id, charmer_id)],
            },
        )
        .expect("declare Bears as blocker");

    pass_until(&mut engine, |e| {
        !stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    tap_all_mana(&mut engine, p1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine
        .apply(p1, PlayerAction::YesNo(true))
        .expect("pay {2} to prevent life gain");

    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(engine.state().players[0].life, 20);
}
