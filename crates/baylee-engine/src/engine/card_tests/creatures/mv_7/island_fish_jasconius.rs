//! `cards/creatures/mv_7/island_fish_jasconius.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Island Fish Jasconius — the first sentence, "This creature doesn't untap
/// during your untap step" (CR 502.3), against the second, "At the beginning
/// of your upkeep, you may pay {U}{U}{U}. If you do, untap this creature."
/// A Forest beside it is the counter-check: at p0's next untap step the
/// Forest untaps and the Fish does not, and declining the upkeep payment
/// leaves it that way.
#[test]
fn island_fish_jasconius_stays_tapped_when_the_upkeep_payment_is_declined() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island_fish_jasconius(), island(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);

    // The turn-one upkeep asks before the Fish can be tapped, so decline it
    // there and walk on to the main phase.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
            && stack_is_empty(e)
    });

    let fish = on_battlefield(&engine, p0, island_fish_jasconius()).expect("seated");
    let forest_id = on_battlefield(&engine, p0, forest()).expect("the counter-check");
    {
        let state = engine
            .dev_state_mut(p0)
            .expect("the harness may set boards up");
        state.set_tapped(fish, true);
        state.set_tapped(forest_id, true);
    }
    engine.refresh_offer();

    // The next question is the Fish's upkeep again, on the far side of an
    // untap step: the Forest is up and the Fish is down.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    assert!(
        !is_tapped(&engine, forest_id),
        "the untap step ran (CR 502.3)"
    );
    assert!(is_tapped(&engine, fish), "and the Fish did not untap");

    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
            && stack_is_empty(e)
    });
    assert!(
        is_tapped(&engine, fish),
        "the declined payment bought no untap"
    );
    assert!(
        on_battlefield(&engine, p0, island_fish_jasconius()).is_some(),
        "and no sacrifice: p0 still controls Islands"
    );
}

/// The same upkeep clause, the other answer: with three Islands up the
/// {U}{U}{U} is paid (CR 608.2d, the question is put as the ability
/// resolves) and the Fish untaps even though its own first sentence would
/// have kept it down.
#[test]
fn island_fish_jasconius_untaps_when_uuu_is_paid() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island_fish_jasconius(), island(), island(), island()])
        .start();
    keep_mulligans(&mut engine);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
            && stack_is_empty(e)
    });

    let fish = on_battlefield(&engine, p0, island_fish_jasconius()).expect("seated");
    engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up")
        .set_tapped(fish, true);
    engine.refresh_offer();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    assert!(is_tapped(&engine, fish), "the untap step left it down");
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    assert_eq!(
        engine.payment_window(),
        Some((
            p0,
            baylee_core::mana::ManaPayment::Fixed(baylee_core::mana!("{U}{U}{U}"))
        )),
        "the window is for the printed price"
    );
    assert_eq!(
        tap_all_mana(&mut engine, p0),
        3,
        "three Islands, three blue"
    );
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        !is_tapped(&engine, fish),
        "\"if you do, untap this creature\""
    );
}

/// The third sentence, "This creature can't attack unless defending player
/// controls an Island" (CR 508.1c): p1 has none, so the Fish is not in the
/// attacker offer while the Savannah Lions beside it is, and a declaration
/// naming the Fish is refused.
#[test]
fn island_fish_jasconius_cannot_attack_when_the_defender_has_no_island() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island_fish_jasconius(), island(), savannah_lions()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });

    let fish = on_battlefield(&engine, p0, island_fish_jasconius()).expect("seated");
    let lions = on_battlefield(&engine, p0, savannah_lions()).expect("seated");
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("the pass waited for exactly this")
    };
    assert!(
        attackers.contains(&lions),
        "the board can offer an attacker: {attackers:?}"
    );
    assert!(
        !attackers.contains(&fish),
        "no Island on the defending side, and its own controller's Islands \
         do not count: {attackers:?}"
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::DeclareAttackers {
                    attackers: vec![(fish, Defender::Player(p1))]
                }
            )
            .is_err(),
        "a declaration naming it is refused"
    );
}

/// The same sentence from the other side: one Island under p1 and the Fish
/// is offered and attacks.
#[test]
fn island_fish_jasconius_attacks_when_the_defender_controls_an_island() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island_fish_jasconius(), island()])
        .battlefield(1, &[island()])
        .start();
    keep_mulligans(&mut engine);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });

    let fish = on_battlefield(&engine, p0, island_fish_jasconius()).expect("seated");
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("the pass waited for exactly this")
    };
    assert!(
        attackers.contains(&fish),
        "an Island on the defending side offers it: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(fish, Defender::Player(p1))],
            },
        )
        .expect("an Island across the table lets it attack");
    assert!(engine.state().combat.is_attacking(fish));
}

/// The fourth sentence, "When you control no Islands, sacrifice this
/// creature" (CR 603.8): Stone Rain destroys the Fish's last Island and the
/// state trigger resolves into the sacrifice.
#[test]
fn island_fish_jasconius_is_sacrificed_when_its_controller_has_no_islands() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                island_fish_jasconius(),
                island(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[stone_rain()])
        .start();
    keep_mulligans(&mut engine);

    // The turn-one upkeep asks for the untap payment first: decline it, then
    // walk to the main phase.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    reach_main_phase(&mut engine, p0);
    let fish = on_battlefield(&engine, p0, island_fish_jasconius()).expect("seated");
    let isle = on_battlefield(&engine, p0, island()).expect("the one Island");

    cast_from_hand(&mut engine, p0, stone_rain());
    aim_at(&mut engine, p0, isle);

    let before = engine.journal().entries().len();
    pass_until(&mut engine, |e| {
        in_graveyard(e, p0, island_fish_jasconius()).is_some()
    });
    assert!(
        on_battlefield(&engine, p0, island()).is_none(),
        "the last Island is gone"
    );
    assert!(
        in_graveyard(&engine, p0, island_fish_jasconius()).is_some(),
        "\"sacrifice this creature\" put it in its owner's graveyard"
    );
    assert!(
        engine.journal().entries()[before..]
            .iter()
            .any(|e| matches!(
                e.event,
                crate::event::GameEvent::AbilityTriggered {
                    source,
                    ability_index: 3,
                    ..
                } if source == fish
            )),
        "the sacrifice is the state trigger at ability index 3, not Stone Rain itself"
    );
}
