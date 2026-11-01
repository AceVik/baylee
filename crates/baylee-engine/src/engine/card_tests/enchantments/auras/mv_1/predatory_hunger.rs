use super::*;

fn predatory_hunger() -> CardIndex {
    card_index("eed2606b-9a27-4af3-8120-c45a1d6b5c00")
}

#[test]
fn predatory_hunger_puts_p1p1_counter_on_enchanted_creature_when_opponent_casts_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[gray_ogre(), forest()])
        .battlefield(1, &[forest(), mountain()])
        .hand(0, &[predatory_hunger(), llanowar_elves()])
        .hand(1, &[llanowar_elves(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ogre = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, predatory_hunger());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![ogre],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    crate::engine::card_tests::legends_kit::float(&mut engine, p0, &[(ManaColor::Green, 1)]);
    cast_with_floating(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, ogre),
        (2, 2),
        "your own creature does not trigger it"
    );
    reach_their_main_phase(&mut engine, p1);
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, lightning_bolt());
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, ogre),
        (2, 2),
        "an opponents noncreature does not trigger it"
    );
    cast_with_floating(&mut engine, p1, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, ogre), (3, 3));
}
