#[allow(clippy::wildcard_imports)]
use super::*;

fn shivan_raptor() -> CardIndex {
    card_index("bc01d36b-dea3-47d4-afdf-0dbbba7ca7b2")
}

#[test]
fn shivan_raptor_stays_on_battlefield_when_echo_paid() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[shivan_raptor()])
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, shivan_raptor());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, shivan_raptor()).is_some());

    reach_their_main_phase(&mut engine, p0);
    assert!(on_battlefield(&engine, p0, shivan_raptor()).is_some());
}
