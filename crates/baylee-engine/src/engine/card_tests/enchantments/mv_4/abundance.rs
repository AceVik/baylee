#[allow(clippy::wildcard_imports)]
use super::*;

fn abundance() -> CardIndex {
    card_index("eccc9a54-2b56-4a02-9926-258d5b2e25fb")
}

#[test]
fn abundance_resolves_onto_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[abundance()])
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, abundance());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, abundance()).is_some());
}
