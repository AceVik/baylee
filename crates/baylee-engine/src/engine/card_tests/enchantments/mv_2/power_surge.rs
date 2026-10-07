//! `cards/enchantments/mv_2/power_surge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Power Surge pays its printed mana cost and resolves onto the battlefield.
/// Its upkeep behavior is covered separately below.
#[test]
fn alpha_eval_power_surge_supported_cast_and_resolution() {
    let p0 = PlayerId::new(0);
    let card = card_index("156b2228-f7b2-4816-b894-c4953a32c05f");
    let mut engine = Duel::new(1001, forest())
        .battlefield(0, &[mountain(); 2])
        .hand(0, &[card])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, card);
    assert!(on_stack(&engine, card).is_some(), "the card was cast");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the printed cost was paid"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, card).is_some());
    assert!(in_hand(&engine, p0, card).is_none());
}

/// Tapping in response cannot change the historical count, and the other
/// player's upkeep uses their own pre-untap lands, not the source controller's.
#[test]
fn alpha_eval_power_surge_uses_each_turns_pre_untap_count() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let surge = card_index("156b2228-f7b2-4816-b894-c4953a32c05f");
    let mut engine = Duel::new(1010, forest())
        .battlefield(0, &[surge, mountain(), mountain(), mountain()])
        .battlefield(1, &[forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(!stack_is_empty(&engine), "the first upkeep triggers");
    let mountains = all_on_battlefield(&engine, p0, mountain());
    for source in mountains.iter().take(2) {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: *source })
            .unwrap();
    }
    pass_until(&mut engine, |e| at_rest(e, p1));
    let green = on_battlefield(&engine, p1, forest()).unwrap();
    engine
        .apply(p1, PlayerAction::ActivateManaAbility { source: green })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 17);
    assert_eq!(engine.state().players[1].life, 20);
    pass_until(&mut engine, |e| {
        e.state().turn.number == 2 && !stack_is_empty(e)
    });
    assert!(
        !engine
            .state()
            .object(green)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[1].life,
        19,
        "only one Forest was untapped before the turn"
    );
    pass_until(&mut engine, |e| {
        e.state().turn.number == 3 && !stack_is_empty(e)
    });
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        16,
        "two Mountains untapped this turn, one was already untapped"
    );
}

#[test]
fn alpha_eval_power_surge_with_no_untapped_lands_deals_no_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let surge = card_index("156b2228-f7b2-4816-b894-c4953a32c05f");
    let mut engine = Duel::new(1011, forest()).battlefield(0, &[surge]).start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 20);
    pass_until(&mut engine, |e| {
        e.state().turn.number == 2 && !stack_is_empty(e)
    });
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, 20);
    assert!(on_battlefield(&engine, p0, surge).is_some());
    assert!(at_rest(&engine, p1) || at_rest(&engine, p0));
}
