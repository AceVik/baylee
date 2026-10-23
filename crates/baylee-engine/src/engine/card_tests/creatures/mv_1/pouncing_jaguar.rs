#[allow(clippy::wildcard_imports)]
use super::*;

fn pouncing_jaguar() -> CardIndex {
    card_index("70a2b5ea-dae4-46e8-87eb-8c8d8dfb242b")
}

#[test]
fn pouncing_jaguar_stays_on_battlefield_when_echo_paid() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[pouncing_jaguar()])
        .battlefield(0, &[forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, pouncing_jaguar());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, pouncing_jaguar()).is_some());

    reach_their_main_phase(&mut engine, p0);
    assert!(on_battlefield(&engine, p0, pouncing_jaguar()).is_some());
}
