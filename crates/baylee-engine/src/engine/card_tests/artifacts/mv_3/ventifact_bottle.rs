//! `cards/artifacts/mv_3/ventifact_bottle.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn ventifact_bottle() -> CardIndex {
    card_index("8f167e2f-7b89-4ae2-8332-7c978f71522e")
}

/// Ventifact Bottle (`{3}`, `Coverage::Implemented`): `{X}{1}, {T}` sorcery-
/// speed ability putting X charge counters on it; at the beginning of your
/// first main phase, if it has a charge counter, tap it, remove all charge
/// counters, and add `{C}` for each charge counter removed.
#[test]
fn ventifact_bottle_stores_charge_counters_and_drains_them_for_mana_at_first_main() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[ventifact_bottle(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bottle = on_battlefield(&engine, p0, ventifact_bottle()).unwrap();

    // Activate {X}{1}, {T} with X = 2 (total cost 3 mana).
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: bottle,
                ability_index: 0,
            },
        )
        .unwrap();
    engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .object(bottle)
            .unwrap()
            .counters
            .iter()
            .find(|(k, _)| *k == CounterKind::Charge)
            .map_or(0, |(_, n)| n),
        2
    );

    // Advance to p0's next first main phase: trigger taps Ventifact Bottle, removes all charge counters, and adds {C}{C}.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, bottle));
    assert_eq!(
        engine
            .state()
            .object(bottle)
            .unwrap()
            .counters
            .iter()
            .find(|(k, _)| *k == CounterKind::Charge)
            .map_or(0, |(_, n)| n),
        0
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        2
    );
}

/// Ventifact Bottle — "if this artifact has a charge counter on it": with
/// none, the first main phase taps nothing and adds no mana.
#[test]
fn ventifact_bottle_without_a_charge_counter_does_nothing_at_first_main() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[ventifact_bottle()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bottle = on_battlefield(&engine, p0, ventifact_bottle()).unwrap();
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        !is_tapped(&engine, bottle),
        "nothing to drain, nothing tapped"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        0,
        "no counter, no {{C}}"
    );
}
