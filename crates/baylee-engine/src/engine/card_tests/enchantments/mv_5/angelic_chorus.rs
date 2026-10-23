#[allow(clippy::wildcard_imports)]
use super::*;

fn angelic_chorus() -> CardIndex {
    card_index("e66b12ff-45da-46dc-9148-bd8ea9b7fde0")
}

#[test]
fn angelic_chorus_resolves_onto_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[angelic_chorus()])
        .battlefield(0, &[plains(), plains(), plains(), plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, angelic_chorus());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, angelic_chorus()).is_some());
}
