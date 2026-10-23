#[allow(clippy::wildcard_imports)]
use super::*;

fn okk() -> CardIndex {
    card_index("f835a8d3-e117-4669-9952-6e9ce13b35fa")
}

#[test]
fn okk_enters_as_4_4_goblin() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[okk()])
        .battlefield(0, &[mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, okk());
    pass_until(&mut engine, stack_is_empty);

    let perm = on_battlefield(&engine, p0, okk()).expect("seated");
    assert_eq!(pt(&engine, perm), (4, 4));
}
