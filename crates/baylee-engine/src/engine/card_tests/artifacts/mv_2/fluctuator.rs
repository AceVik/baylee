#[allow(clippy::wildcard_imports)]
use super::*;

fn fluctuator() -> CardIndex {
    card_index("5ed0c7cc-9969-4387-90f8-3084dd18026b")
}

#[test]
fn fluctuator_enters_battlefield_as_artifact() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[fluctuator()])
        .battlefield(0, &[forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, fluctuator());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, fluctuator()).is_some());
}
