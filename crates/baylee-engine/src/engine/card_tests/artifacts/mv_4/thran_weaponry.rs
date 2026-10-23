#[allow(clippy::wildcard_imports)]
use super::*;

fn thran_weaponry() -> CardIndex {
    card_index("1af303ef-dd76-4684-97a1-355b7eee53e4")
}

#[test]
fn thran_weaponry_pumps_all_creatures_while_tapped() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[gray_ogre()])
        .hand(0, &[thran_weaponry()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_from_hand(&mut engine, p0, thran_weaponry());
    pass_until(&mut engine, stack_is_empty);

    let weaponry = on_battlefield(&engine, p0, thran_weaponry()).unwrap();
    let elves = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: weaponry,
                ability_index: 2,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, elves), (3, 3));
    assert_eq!(pt(&engine, ogre), (4, 4));
}
