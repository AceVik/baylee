//! `cards/creatures/mv_3/viashino_grappler.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Viashino Grappler` is a 3/1 creature costing `{2}{R}` under `Coverage::Implemented`.
/// It prints "{G}: This creature gains trample until end of turn."
/// When activated off a Forest for `{G}`, the ability resolves without tapping the creature
/// and grants it trample until end of turn.
#[test]
fn viashino_grappler_gains_trample_until_end_of_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[viashino_grappler(), forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let grappler = on_battlefield(&engine, p0, viashino_grappler())
        .expect("Viashino Grappler is on the battlefield");
    assert_eq!(pt(&engine, grappler), (3, 1), "base body is 3/1");
    assert!(
        !keywords(&engine, grappler).contains(KeywordSet::TRAMPLE),
        "does not have trample initially"
    );

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, viashino_grappler(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, grappler).contains(KeywordSet::TRAMPLE),
        "Viashino Grappler gains trample until end of turn"
    );
    assert!(
        !is_tapped(&engine, grappler),
        "activation cost did not require tapping the creature"
    );
}
