#[allow(clippy::wildcard_imports)]
use super::*;

fn titania_s_boon() -> CardIndex {
    card_index("0bf4c01d-3cb0-47be-b84a-6a399bc2d3fc")
}

#[test]
fn titania_s_boon_puts_plus_one_counter_on_each_creature_you_control() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[titania_s_boon()])
        .battlefield(
            0,
            &[
                gray_ogre(),
                wild_elephant(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("ogre on bf");
    let elephant = on_battlefield(&engine, p0, wild_elephant()).expect("elephant on bf");
    cast_from_hand(&mut engine, p0, titania_s_boon());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, ogre), (3, 3));
    assert_eq!(pt(&engine, elephant), (4, 4));
}
