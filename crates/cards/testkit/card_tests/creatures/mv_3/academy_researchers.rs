#[allow(clippy::wildcard_imports)]
use super::*;

fn academy_researchers() -> CardIndex {
    card_index("df2d2c52-475b-4057-9cf8-7886700f0fae")
}

#[test]
fn academy_researchers_enters_as_two_two_human_wizard() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[academy_researchers()])
        .battlefield(0, &[island(), island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, academy_researchers());
    pass_until(&mut engine, stack_is_empty);

    let obj = on_battlefield(&engine, p0, academy_researchers()).expect("seated");
    assert_eq!(pt(&engine, obj), (2, 2));
}
