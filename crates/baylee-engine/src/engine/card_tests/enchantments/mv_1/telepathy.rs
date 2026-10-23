#[allow(clippy::wildcard_imports)]
use super::*;

fn telepathy() -> CardIndex {
    card_index("d2e647ac-717f-46e6-baa8-6a793e7bdc32")
}

#[test]
fn telepathy_reveals_hands_while_on_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[telepathy()])
        .battlefield(0, &[island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, telepathy());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, telepathy()).is_some());
}
