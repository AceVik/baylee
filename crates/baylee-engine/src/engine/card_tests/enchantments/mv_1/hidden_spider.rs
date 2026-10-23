#[allow(clippy::wildcard_imports)]
use super::*;

fn hidden_spider() -> CardIndex {
    card_index("b0709570-a145-4c3c-ba56-f06d25753855")
}

fn fog_bank() -> CardIndex {
    card_index("3a2bb47d-228d-499e-a34b-9e10d99e9b2c")
}

#[test]
fn hidden_spider_becomes_3_5_spider_with_reach_when_opponent_casts_flyer() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[hidden_spider()])
        .hand(1, &[fog_bank()])
        .battlefield(1, &[island(), island()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p1));

    cast_from_hand(&mut engine, p1, fog_bank());
    pass_until(&mut engine, stack_is_empty);

    let perm = on_battlefield(&engine, p0, hidden_spider()).expect("seated");
    assert_eq!(pt(&engine, perm), (3, 5));
}
