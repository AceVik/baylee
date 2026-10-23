#[allow(clippy::wildcard_imports)]
use super::*;

fn defensive_formation() -> CardIndex {
    card_index("9386f4d3-aa72-4849-8921-83740762dc0e")
}

#[test]
fn defensive_formation_resolves_onto_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[defensive_formation()])
        .battlefield(0, &[plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, defensive_formation());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, defensive_formation()).is_some());
}
