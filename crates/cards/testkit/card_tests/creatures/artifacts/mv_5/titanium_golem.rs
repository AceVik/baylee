//! `cards/creatures/artifacts/mv_5/titanium_golem.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Titanium Golem` prints `{{1}}{{W}}: This creature gains first strike until end of turn.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls the 3/3 artifact creature and two Plains.
/// Tapping the Plains provides the necessary mana to activate the ability.
/// Upon resolution, `Titanium Golem` gains `KeywordSet::FIRST_STRIKE` until end of turn.
#[test]
fn titanium_golem_gains_first_strike_until_end_of_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[titanium_golem(), plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let golem = on_battlefield(&engine, p0, titanium_golem()).expect("titanium golem is seated");
    assert_eq!(pt(&engine, golem), (3, 3));
    assert!(
        !keywords(&engine, golem).contains(KeywordSet::FIRST_STRIKE),
        "starts without first strike"
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);

    activate(&mut engine, p0, titanium_golem(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, golem).contains(KeywordSet::FIRST_STRIKE),
        "`Titanium Golem` gains first strike upon resolution"
    );
    assert_eq!(pt(&engine, golem), (3, 3));
}
