#[allow(clippy::wildcard_imports)]
use super::*;

fn granite_grip() -> CardIndex {
    card_index("19e09405-3294-4f98-a18f-60cf0a8a2e4e")
}

#[test]
fn granite_grip_gives_enchanted_creature_plus_one_plus_zero_per_mountain() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[granite_grip()])
        .battlefield(0, &[mountain(), mountain(), mountain(), forest()])
        .battlefield(1, &[grizzly_bears(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, PlayerId::new(1), grizzly_bears()).unwrap();
    cast_from_hand(&mut engine, p0, granite_grip());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bear],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bear), (5, 2));
}
