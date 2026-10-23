#[allow(clippy::wildcard_imports)]
use super::*;

fn goblin_patrol() -> CardIndex {
    card_index("edf25cbe-fa28-44ca-b4be-1d2312e4f6ac")
}

#[test]
fn goblin_patrol_enters_as_2_1_with_echo() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[goblin_patrol()])
        .battlefield(0, &[mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, goblin_patrol());
    pass_until(&mut engine, stack_is_empty);

    let perm = on_battlefield(&engine, p0, goblin_patrol()).expect("seated");
    assert_eq!(pt(&engine, perm), (2, 1));
}
