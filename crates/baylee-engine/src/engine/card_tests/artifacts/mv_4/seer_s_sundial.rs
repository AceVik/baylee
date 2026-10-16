//! `cards/artifacts/mv_4/seer_s_sundial.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn seer_s_sundial() -> CardIndex {
    card_index("9929d7ba-cdc5-4099-a0ee-3a15073336f3")
}

#[test]
fn probe() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[seer_s_sundial(), forest(), forest()])
        .hand(0, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    play_land(&mut engine, p0, forest());
    pass_until(&mut engine, |e| !matches!(e.pending(), Pending::Priority { .. }) || stack_is_empty(e));
    panic!("PROBE {:?}", engine.pending());
}
