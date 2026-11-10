//! `cards/creatures/artifacts/mv_4/patagia_golem.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Patagia Golem` prints `{{3}}: This creature gains flying until end of turn.` on a 2/3
/// artifact creature with `Coverage::Implemented`.
/// In this scenario, three `forest()` lands float the required generic mana to activate the ability.
/// Upon resolution, the layer system recomputes the characteristics, granting `KeywordSet::FLYING`
/// while maintaining its base 2/3 power and toughness.
#[test]
fn patagia_golem_gains_flying_until_end_of_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), patagia_golem()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let golem = on_battlefield(&engine, p0, patagia_golem()).expect("golem is seated");
    assert_eq!(pt(&engine, golem), (2, 3));
    assert!(!keywords(&engine, golem).contains(KeywordSet::FLYING));

    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 3);

    activate(&mut engine, p0, patagia_golem(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, golem), (2, 3));
    assert!(keywords(&engine, golem).contains(KeywordSet::FLYING));
}
