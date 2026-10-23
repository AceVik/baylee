#[allow(clippy::wildcard_imports)]
use super::*;

fn multani_s_presence() -> CardIndex {
    card_index("adad1be7-8d3d-4857-a3b0-be33db4c5746")
}

#[test]
fn multani_s_presence_enters_the_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[multani_s_presence()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, multani_s_presence());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, multani_s_presence()).is_some());
}
