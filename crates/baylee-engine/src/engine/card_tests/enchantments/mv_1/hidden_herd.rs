#[allow(clippy::wildcard_imports)]
use super::*;

fn hidden_herd() -> CardIndex {
    card_index("8e7c5649-41c2-4a0c-b873-e6f6c5908567")
}

#[test]
fn hidden_herd_becomes_3_3_beast_when_opponent_plays_land() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[hidden_herd()])
        .hand(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p1));

    play_land(&mut engine, p1, forest());
    pass_until(&mut engine, stack_is_empty);

    let perm = on_battlefield(&engine, p0, hidden_herd()).expect("seated");
    assert_eq!(pt(&engine, perm), (3, 3));
}
