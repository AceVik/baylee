//! `cards/enchantments/auras/mv_2/exoskeletal_armor.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn exoskeletal_armor_pumps_enchanted_creature_by_creature_cards_in_all_graveyards() {
    let armor = card_index("29358dcd-b26a-4720-9daa-7baa648e3540");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                grizzly_bears(),
                gray_ogre(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[grizzly_bears(), forest()])
        .hand(0, &[armor])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let p0_dead = [
        on_battlefield(&engine, p0, gray_ogre()).unwrap(),
        on_battlefield(&engine, p0, llanowar_elves()).unwrap(),
    ];
    let p1_dead = [
        on_battlefield(&engine, p1, grizzly_bears()).unwrap(),
        on_battlefield(&engine, p1, forest()).unwrap(),
    ];
    bury(&mut engine, &p0_dead);
    bury(&mut engine, &p1_dead);

    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    cast_from_hand(&mut engine, p0, armor);
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

    // 3 creature cards across both graveyards (Gray Ogre + Llanowar Elves + Grizzly Bears; the buried Forest is not a creature).
    assert_eq!(pt(&engine, bear), (5, 5));
}
