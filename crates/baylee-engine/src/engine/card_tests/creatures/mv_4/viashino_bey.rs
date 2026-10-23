#[allow(clippy::wildcard_imports)]
use super::*;

fn viashino_bey() -> CardIndex {
    card_index("d9c9b933-74f4-4898-bbb8-2da76b86ccda")
}

#[test]
fn viashino_bey_casts_as_four_three_lizard() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[viashino_bey()])
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, viashino_bey());
    pass_until(&mut engine, stack_is_empty);

    let bey = on_battlefield(&engine, p0, viashino_bey()).unwrap();
    assert_eq!(pt(&engine, bey), (4, 3));
}
