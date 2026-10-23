#[allow(clippy::wildcard_imports)]
use super::*;

fn bubbling_beebles() -> CardIndex {
    card_index("75903c3b-2dbe-4662-b6a8-39599e49adde")
}

#[test]
fn bubbling_beebles_casts_for_four_and_blue_as_a_three_three() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[bubbling_beebles()])
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, bubbling_beebles());
    pass_until(&mut engine, stack_is_empty);

    let beebles = on_battlefield(&engine, p0, bubbling_beebles()).unwrap();
    assert_eq!(pt(&engine, beebles), (3, 3));
}
