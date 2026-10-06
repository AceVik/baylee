//! `cards/creatures/artifacts/mv_4/cobalt_golem.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Cobalt Golem` prints `{{1}}{{U}}: This creature gains flying until end of turn.`
/// on a 2/3 artifact creature with `Coverage::Implemented`.
/// In this scenario, two `island()` lands float the required mana to activate its ability.
/// Upon resolution, the layer system recomputes the golem's characteristics and grants
/// `KeywordSet::FLYING` while preserving its 2/3 base stats.
#[test]
fn cobalt_golem_gains_flying_until_end_of_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), cobalt_golem()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let golem = on_battlefield(&engine, p0, cobalt_golem()).expect("golem is seated");
    assert_eq!(pt(&engine, golem), (2, 3));
    assert!(!keywords(&engine, golem).contains(KeywordSet::FLYING));

    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);

    activate(&mut engine, p0, cobalt_golem(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, golem), (2, 3));
    assert!(keywords(&engine, golem).contains(KeywordSet::FLYING));
}
