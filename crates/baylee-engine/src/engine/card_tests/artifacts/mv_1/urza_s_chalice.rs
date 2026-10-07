//! `cards/artifacts/mv_1/urza_s_chalice.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Urza's Chalice: "Whenever a player casts an artifact spell, you may pay
/// {1}. If you do, you gain 1 life."
///
/// "A player" is any player (CR 603.6a watches the cast, not the caster),
/// so p1's Sol Ring puts the question to p0, the Chalice's controller
/// (CR 109.5: "you" is the ability's controller). The {1} is made in the
/// CR 605.3a window the yes opens — p0's Forest is untapped and the pool is
/// empty — so the payment is read off the Forest being tapped and the pool
/// emptied, not just off the life total.
#[test]
fn urza_s_chalice_gains_a_life_for_an_opponents_artifact_spell_once_paid() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[urza_s_chalice(), forest()])
        .battlefield(1, &[island()])
        .hand(1, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    let forest = on_battlefield(&engine, p0, forest()).expect("the Forest is out");
    cast_from_hand(&mut engine, p1, quiet_artifact());

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { mana: 1 },
                ..
            }
        )
    });
    let Pending::YesNo { player, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on the question")
    };
    assert_eq!(player, p0, "the Chalice's controller is the one asked");
    assert!(!is_tapped(&engine, forest), "nothing is floating yet");
    let before = life_of(&engine, p0);

    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    assert_eq!(
        engine.payment_window(),
        Some((
            p0,
            baylee_core::mana::ManaPayment::Fixed(baylee_core::mana!("{1}"))
        )),
        "an empty pool: the yes opens the window for the printed {{1}}"
    );
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: forest })
        .unwrap();
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        life_of(&engine, p0),
        before + 1,
        "paid {{1}}, gained 1 life"
    );
    assert!(is_tapped(&engine, forest), "the Forest paid the price");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool is empty"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "the trigger never countered the spell it heard"
    );
}

/// The declined half, on the Chalice's own controller's cast: "a player" is
/// not only the opponent, and a no spends nothing and gains nothing. The
/// floating {{G}} is read on both sides of the answer, so "nothing was paid"
/// is a claim about the pool and not only about the life total.
#[test]
fn urza_s_chalice_offers_its_tax_on_its_own_controllers_artifact_spell() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[urza_s_chalice(), forest(), forest()])
        .hand(0, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let before = life_of(&engine, p0);
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, quiet_artifact());
    let pool = engine.state().players[0].mana_pool.total();
    assert_eq!(pool, 1, "the {{1}} spell left one green floating");

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { mana: 1 },
                ..
            }
        )
    });
    let Pending::YesNo { player, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on the question")
    };
    assert_eq!(player, p0, "your own cast triggers your own Chalice");
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();

    assert_eq!(life_of(&engine, p0), before, "a no gains nothing");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        pool,
        "and spends nothing"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
        "the Ring resolved"
    );
}

/// The 2004 ruling: "This will not trigger on its own casting. It must be
/// on the battlefield at the time the artifact is cast." Casting the Chalice
/// asks nothing, and the walk would panic on a `PayTax` question its match
/// has no arm for, so the silence is asserted rather than assumed.
#[test]
fn urza_s_chalice_does_not_trigger_on_its_own_casting() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[urza_s_chalice()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, urza_s_chalice());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, p0), 20, "no life was offered or gained");
    assert!(on_battlefield(&engine, p0, urza_s_chalice()).is_some());
}
