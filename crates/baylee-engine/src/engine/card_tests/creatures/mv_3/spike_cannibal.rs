#[allow(clippy::wildcard_imports)]
use super::*;

fn spike_cannibal() -> CardIndex {
    card_index("c175ab0e-7085-41ec-83e4-38fc85c338fa")
}

#[test]
fn spike_cannibal_enters_with_one_plus_one_plus_one_counter() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[spike_cannibal()])
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, spike_cannibal());
    pass_until(&mut engine, stack_is_empty);
    let cannibal = on_battlefield(&engine, p0, spike_cannibal()).expect("seated");
    assert_eq!(pt(&engine, cannibal), (1, 1));
}
