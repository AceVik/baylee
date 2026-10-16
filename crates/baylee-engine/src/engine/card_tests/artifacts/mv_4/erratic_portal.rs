//! `cards/artifacts/mv_4/erratic_portal.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn erratic_portal() -> CardIndex {
    card_index("4a85cd7e-625b-4043-b7f1-999def26eb36")
}

#[test]
fn probe() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[erratic_portal(), forest(), forest()])
        .battlefield(1, &[gray_ogre(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, erratic_portal(), 0);
    aim_at(&mut engine, p0, ogre);
    pass_until(&mut engine, |e| !matches!(e.pending(), Pending::Priority { .. }) || stack_is_empty(e));
    panic!("PROBE {:?}", engine.pending());
}
