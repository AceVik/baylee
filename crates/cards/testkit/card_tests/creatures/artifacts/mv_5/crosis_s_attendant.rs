//! `cards/creatures/artifacts/mv_5/crosis_s_attendant.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Crosis's Attendant` prints `{{1}}, Sacrifice this creature: Add {{U}}{{B}}{{R}}.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls the 3/3 golem and one Forest.
/// Tapping the Forest provides the generic mana required to activate the ability.
/// As a mana ability, it resolves immediately without using the stack, sacrificing itself and adding one blue, one black, and one red mana.
#[test]
fn crosis_s_attendant_sacrifices_for_grixis_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[crosis_s_attendant(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let attendant = on_battlefield(&engine, p0, crosis_s_attendant()).expect("attendant seated");
    assert_eq!(pt(&engine, attendant), (3, 3));

    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);

    activate(&mut engine, p0, crosis_s_attendant(), 0);
    assert!(
        stack_is_empty(&engine),
        "mana ability does not use the stack"
    );
    assert!(
        on_battlefield(&engine, p0, crosis_s_attendant()).is_none(),
        "attendant was sacrificed"
    );
    assert!(
        in_graveyard(&engine, p0, crosis_s_attendant()).is_some(),
        "attendant is in the graveyard"
    );

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1, "produced one blue");
    assert_eq!(pool.available(ManaColor::Black), 1, "produced one black");
    assert_eq!(pool.available(ManaColor::Red), 1, "produced one red");
    assert_eq!(pool.available(ManaColor::Green), 0, "green mana was spent");
    assert_eq!(pool.total(), 3, "total three mana floating");
}
