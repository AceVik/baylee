#[allow(clippy::wildcard_imports)]
use super::*;

fn cradle_guard() -> CardIndex {
    card_index("846c0867-6f0e-4e44-8f45-8f9720b62065")
}

#[test]
fn cradle_guard_casts_with_trample() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[cradle_guard()])
        .battlefield(0, &[forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, cradle_guard());
    pass_until(&mut engine, stack_is_empty);

    let guard = on_battlefield(&engine, p0, cradle_guard()).expect("seated");
    assert!(keywords(&engine, guard).contains(KeywordSet::TRAMPLE));
}
