//! `cards/creatures/mv_2/hasran_ogress.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Hasran Ogress — `{B}{B}` 3/2 Ogre: "Whenever this creature attacks, it
/// deals 3 damage to you unless you pay {2}."
///
/// Declining is the damage. The price is asked of the *attacking* seat (the
/// "you" the damage is dealt to), the two Swamps are left untapped so the
/// pool could pay and declining is a choice rather than an inability
/// (CR 118.12a), and the 3 goes to its controller's life rather than the
/// defending player's — which is the half a `PlayerRel` can get backwards
/// without anything else noticing.
#[test]
fn hasran_ogress_deals_three_to_its_controller_when_the_attack_is_not_paid_for() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[hasran_ogress(), swamp(), swamp()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ogress = on_battlefield(&engine, p0, hasran_ogress()).expect("seated");
    assert_eq!(pt(&engine, ogress), (3, 2), "the printed body");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(ogress, Defender::Player(p1))],
            },
        )
        .expect("an untapped 3/2 with no restriction is on the offer");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on the question")
    };
    assert_eq!(
        player, p0,
        "\"deals 3 damage to *you*\": the attacking seat is the one asked"
    );
    assert_eq!(prompt, YesNoPrompt::PayTax { mana: 2 }, "the printed {{2}}");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats yet, and the untapped Swamps still make declining a choice"
    );

    engine
        .apply(p0, PlayerAction::YesNo(false))
        .expect("declining is an answer out of the question's own enumeration");
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert_eq!(
        engine.state().players[0].life,
        17,
        "the unpaid attack dealt its printed 3 to its controller"
    );
    assert_eq!(
        engine.state().players[1].life,
        17,
        "and the unblocked 3/2 still connected for its printed 3"
    );
    assert!(
        on_battlefield(&engine, p0, hasran_ogress()).is_some(),
        "the printed sentence is damage, not a sacrifice: the Ogre is standing"
    );
}

/// The paid half: the {2} is made in the CR 605.3a window the yes opened,
/// the fallback never runs, the controller's life stays where it was, and
/// the attack itself still deals its printed 3.
#[test]
fn hasran_ogress_may_buy_its_attack_for_two() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[hasran_ogress(), swamp(), swamp()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ogress = on_battlefield(&engine, p0, hasran_ogress()).expect("seated");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(ogress, Defender::Player(p1))],
            },
        )
        .expect("an untapped 3/2 with no restriction is on the offer");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    assert_eq!(
        engine.payment_window(),
        Some((
            p0,
            baylee_core::mana::ManaPayment::Fixed(baylee_core::mana!("{2}"))
        )),
        "the window is for the printed price"
    );
    tap_all_mana(&mut engine, p0);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert_eq!(
        engine.state().players[0].life,
        20,
        "the {{2}} bought the damage off"
    );
    assert_eq!(
        engine.state().players[1].life,
        17,
        "and the attack still dealt its printed 3"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "both Swamps were spent on the price"
    );
    assert!(on_battlefield(&engine, p0, hasran_ogress()).is_some());
}
