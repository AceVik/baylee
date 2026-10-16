//! `cards/artifacts/mv_4/unbender_tine.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn unbender_tine() -> CardIndex {
    card_index("cdc0a1a5-d9e4-4923-b472-2fa78212167f")
}

/// "{T}: Untap another target permanent." The Tine is not on its own menu,
/// permanents of either seat are, and the named one is untapped.
#[test]
fn it_untaps_another_target_permanent() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[unbender_tine(), forest()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let tine = on_battlefield(&engine, p0, unbender_tine()).unwrap();
    let mine = on_battlefield(&engine, p0, forest()).unwrap();
    let theirs = on_battlefield(&engine, p1, forest()).unwrap();
    {
        let state = engine.dev_state_mut(p0).unwrap();
        state.set_tapped(mine, true);
        state.set_tapped(theirs, true);
    }
    engine.refresh_offer();

    activate(&mut engine, p0, unbender_tine(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("a target, got {:?}", engine.pending())
    };
    assert!(options.contains(&mine) && options.contains(&theirs), "{options:?}");
    assert!(!options.contains(&tine), "\"another\": {options:?}");
    aim_at(&mut engine, p0, theirs);
    assert!(is_tapped(&engine, tine), "{{T}} is the price");
    pass_until(&mut engine, stack_is_empty);
    assert!(!is_tapped(&engine, theirs), "the target untapped");
    assert!(is_tapped(&engine, mine), "and only the target");
}
