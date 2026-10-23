#[allow(clippy::wildcard_imports)]
use super::*;

fn mobile_fort() -> CardIndex {
    card_index("0547e426-e23e-4c60-ba6b-cfd4b5ff8485")
}

#[test]
fn mobile_fort_gets_plus_three_minus_one_when_activated() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mobile_fort(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let fort = on_battlefield(&engine, p0, mobile_fort()).expect("seated");
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, mobile_fort(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, fort), (3, 5));
}
