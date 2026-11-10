//! `cards/creatures/artifacts/mv_5/rith_s_attendant.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Rith's Attendant` prints `{{1}}, Sacrifice this creature: Add {{R}}{{G}}{{W}}.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls the 3/3 golem and one Forest.
/// Tapping the Forest provides the generic mana for the cost.
/// Activating the mana ability resolves immediately without the stack, sacrificing itself and adding one red, one green, and one white mana.
#[test]
fn rith_s_attendant_sacrifices_for_naya_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[rith_s_attendant(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let attendant = on_battlefield(&engine, p0, rith_s_attendant()).expect("attendant seated");
    assert_eq!(pt(&engine, attendant), (3, 3));

    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);

    activate(&mut engine, p0, rith_s_attendant(), 0);
    assert!(
        stack_is_empty(&engine),
        "mana ability does not use the stack"
    );
    assert!(
        on_battlefield(&engine, p0, rith_s_attendant()).is_none(),
        "attendant was sacrificed"
    );
    assert!(
        in_graveyard(&engine, p0, rith_s_attendant()).is_some(),
        "attendant is in the graveyard"
    );

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1, "produced one red");
    assert_eq!(pool.available(ManaColor::Green), 1, "produced one green");
    assert_eq!(pool.available(ManaColor::White), 1, "produced one white");
    assert_eq!(pool.available(ManaColor::Colorless), 0);
    assert_eq!(pool.total(), 3, "total three mana floating");
}
