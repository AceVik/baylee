//! `cards/creatures/mv_3/horseshoe_crab.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Horseshoe Crab is a {2}{U} 1/3 whose entire printed text is "{U}: Untap
/// this creature." — and on an untapped creature that sentence is invisible, so
/// the scenario plays the card the way the card is played: the 1/3 attacks, and
/// one blue stands it back up while the combat it is still part of carries on.
/// `legal.abilities` is filtered through `can_afford`, which reads the mana
/// pool and not the untapped Islands, so the offer is read on both sides of the
/// price — absent on an empty pool, present the moment three Islands have paid
/// in — and the pool is read a last time to say exactly one of those three blue
/// went into the untap.
#[test]
fn horseshoe_crab_attacks_and_stands_back_up_for_one_blue() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), horseshoe_crab()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches their own main"
    );

    let crab = on_battlefield(&engine, p0, horseshoe_crab()).expect("the Crab is on the table");
    assert_eq!(pt(&engine, crab), (1, 3), "the body the card prints");
    assert!(!is_tapped(&engine, crab), "and it arrives untapped");

    // Nothing is floating, and `legal.abilities` is filtered through
    // `can_afford`, which reads the pool rather than the untapped Islands.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("p0 holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(crab, 0)),
        "an empty pool pays no {{U}}, so the untap is not offered: {:?}",
        legal.abilities
    );

    // The card's own line: attack with it, because a tapped Crab is the only
    // Crab on which an untap can be read.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on the attack declaration")
    };
    assert!(
        attackers.contains(&crab),
        "an untapped 1/3 with no clause against it may attack: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(crab, Defender::Player(p1))],
            },
        )
        .unwrap();
    assert!(
        is_tapped(&engine, crab),
        "declaring it as an attacker is what taps it"
    );

    // The mana is tapped only here: a pool is emptied when a step ends
    // (CR 500.5), and the precombat main phase is already behind us.
    pass_until(&mut engine, |e| {
        is_tapped(e, crab)
            && stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        3,
        "three Islands, and the Crab prints no mana ability of its own"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(crab, 0)),
        "with {{U}} floating, the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, horseshoe_crab(), 0);
    assert!(
        !stack_is_empty(&engine),
        "untapping is no mana ability, so the ability waits on the stack"
    );
    assert!(
        is_tapped(&engine, crab),
        "and has not happened yet: the {{U}} is paid on announcement \
         (CR 601.2h) while the effect resolves off the stack"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        !is_tapped(&engine, crab),
        "\"{{U}}: Untap this creature.\" — the Crab is standing again"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        2,
        "exactly one of the three blue went into the untap"
    );

    // It is still the attacking creature the tap paid for: untapping does not
    // remove it from combat, so the damage is still coming.
    pass_until(&mut engine, |e| e.state().players[1].life == 19);
    assert!(
        !is_tapped(&engine, crab),
        "and the 1/3 is still standing at the end of the combat it attacked in"
    );
}
