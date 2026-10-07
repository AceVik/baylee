//! `cards/creatures/artifacts/mv_5/mantis_engine.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Mantis Engine` prints `{{2}}: This creature gains flying until end of turn.` and `{{2}}: This creature gains first strike until end of turn.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Mantis Engine` and four Forests.
/// Activating ability 0 grants `KeywordSet::FLYING`, and activating ability 1 grants `KeywordSet::FIRST_STRIKE`,
/// demonstrating both independent activated pump abilities on the artifact creature.
#[test]
fn mantis_engine_activates_flying_and_first_strike() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[mantis_engine(), forest(), forest(), forest(), forest()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let insect = on_battlefield(&engine, p0, mantis_engine()).expect("mantis engine seated");
    assert_eq!(pt(&engine, insect), (3, 3));
    let initial_kw = keywords(&engine, insect);
    assert!(!initial_kw.contains(KeywordSet::FLYING));
    assert!(!initial_kw.contains(KeywordSet::FIRST_STRIKE));

    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 4);

    activate(&mut engine, p0, mantis_engine(), 0);
    pass_until(&mut engine, stack_is_empty);
    let after_first = keywords(&engine, insect);
    assert!(
        after_first.contains(KeywordSet::FLYING),
        "ability 0 granted flying"
    );
    assert!(
        !after_first.contains(KeywordSet::FIRST_STRIKE),
        "ability 1 was not yet activated"
    );

    activate(&mut engine, p0, mantis_engine(), 1);
    pass_until(&mut engine, stack_is_empty);
    let after_second = keywords(&engine, insect);
    assert!(
        after_second.contains(KeywordSet::FLYING),
        "flying persists until end of turn"
    );
    assert!(
        after_second.contains(KeywordSet::FIRST_STRIKE),
        "ability 1 granted first strike"
    );
}
