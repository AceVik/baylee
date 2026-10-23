#[allow(clippy::wildcard_imports)]
use super::*;

fn radiant_s_dragoons() -> CardIndex {
    card_index("a0fe91a8-633c-4902-bc75-da0a5aa76992")
}

#[test]
fn radiant_s_dragoons_gains_5_life_on_etb() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .hand(0, &[radiant_s_dragoons()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, radiant_s_dragoons());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 25);
}
