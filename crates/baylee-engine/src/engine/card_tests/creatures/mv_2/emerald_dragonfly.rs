//! `cards/creatures/mv_2/emerald_dragonfly.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Emerald Dragonfly` prints `KeywordSet::FLYING` and an activated ability `{{G}}{{G}}`
/// granting first strike until end of turn under `Coverage::Implemented`.
/// Activating the ability on an open board of two Forests verifies that `KeywordSet::FIRST_STRIKE`
/// is successfully added alongside the printed `KeywordSet::FLYING`.
#[test]
fn emerald_dragonfly_activates_first_strike() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1617, forest())
        .battlefield(0, &[emerald_dragonfly(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let dragonfly = on_battlefield(&engine, p0, emerald_dragonfly()).expect("dragonfly is seated");
    assert!(keywords(&engine, dragonfly).contains(KeywordSet::FLYING));
    assert!(!keywords(&engine, dragonfly).contains(KeywordSet::FIRST_STRIKE));

    tap_all_mana(&mut engine, p0);

    activate(&mut engine, p0, emerald_dragonfly(), 0);
    pass_until(&mut engine, stack_is_empty);

    let kw = keywords(&engine, dragonfly);
    assert!(kw.contains(KeywordSet::FLYING));
    assert!(kw.contains(KeywordSet::FIRST_STRIKE));
}
