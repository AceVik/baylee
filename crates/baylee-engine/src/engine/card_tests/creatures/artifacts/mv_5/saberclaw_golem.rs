//! `cards/creatures/artifacts/mv_5/saberclaw_golem.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn saberclaw_golem() -> CardIndex {
    card_index("4989d38e-332f-48f5-863b-1566921156c8")
}

/// Saberclaw Golem — "{R}: This creature gains first strike until end of
/// turn."
#[test]
fn saberclaw_golem_gains_first_strike_for_red() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[saberclaw_golem(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let golem = on_battlefield(&engine, p0, saberclaw_golem()).expect("seated");
    assert_eq!(pt(&engine, golem), (4, 2));
    assert!(!keywords(&engine, golem).contains(KeywordSet::FIRST_STRIKE));
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, saberclaw_golem(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, golem).contains(KeywordSet::FIRST_STRIKE));

    let turn = engine.state().turn.number;
    pass_until(&mut engine, |e| e.state().turn.number > turn);
    assert!(
        !keywords(&engine, golem).contains(KeywordSet::FIRST_STRIKE),
        "until end of turn"
    );
}
