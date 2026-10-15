//! `cards/creatures/artifacts/mv_1/locthwain_gargoyle.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn locthwain_gargoyle() -> CardIndex {
    card_index("d8a04142-1d28-4c2e-b2d5-da88b2199678")
}

/// "{4}: This creature gets +2/+0 and gains flying until end of turn." Four
/// mana buy exactly one activation; both the bonus and flying end with the
/// turn.
#[test]
fn locthwain_gargoyle_gets_power_and_flying_for_four() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[locthwain_gargoyle()], &[]);
    let me = on_battlefield(&engine, p0, locthwain_gargoyle()).expect("seated");
    assert_eq!(pt(&engine, me), (0, 3));
    assert!(!keywords(&engine, me).contains(KeywordSet::FLYING));

    float_mana(&mut engine, ManaColor::Colorless, 3);
    assert!(!is_offered(&engine, me, 0), "three is not {{4}}");
    float_mana(&mut engine, ManaColor::Colorless, 1);
    activate(&mut engine, p0, locthwain_gargoyle(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, me), (2, 3));
    assert!(keywords(&engine, me).contains(KeywordSet::FLYING));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    pass_until(&mut engine, |e| {
        e.state().turn.active == PlayerId::new(1) && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(pt(&engine, me), (0, 3), "until end of turn");
    assert!(!keywords(&engine, me).contains(KeywordSet::FLYING));
}
