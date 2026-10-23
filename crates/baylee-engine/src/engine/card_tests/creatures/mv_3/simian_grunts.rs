#[allow(clippy::wildcard_imports)]
use super::*;

fn simian_grunts() -> CardIndex {
    card_index("158f4e87-60f1-43c1-b03d-9bbe4b824b39")
}

#[test]
fn simian_grunts_casts_with_flash() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[simian_grunts()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, simian_grunts());
    pass_until(&mut engine, stack_is_empty);

    let grunts = on_battlefield(&engine, p0, simian_grunts()).unwrap();
    assert_eq!(pt(&engine, grunts), (3, 4));
}
