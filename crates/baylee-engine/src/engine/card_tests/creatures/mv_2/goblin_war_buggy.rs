#[allow(clippy::wildcard_imports)]
use super::*;

fn goblin_war_buggy() -> CardIndex {
    card_index("59fb1640-d8ed-476a-a78d-91d60b6faac5")
}

#[test]
fn goblin_war_buggy_enters_with_haste_and_echo() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[goblin_war_buggy()])
        .battlefield(0, &[mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, goblin_war_buggy());
    pass_until(&mut engine, stack_is_empty);

    let perm = on_battlefield(&engine, p0, goblin_war_buggy()).expect("seated");
    assert_eq!(pt(&engine, perm), (2, 2));
}
