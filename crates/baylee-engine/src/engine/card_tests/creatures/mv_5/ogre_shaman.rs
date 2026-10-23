use super::*;

fn ogre_shaman() -> CardIndex {
    card_index("04c7bb20-6e40-4e79-a0ec-ced920d3491e")
}

#[test]
fn ogre_shaman_discards_at_random_to_deal_2_damage_to_any_target() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[ogre_shaman(), mountain(), mountain()])
        .hand(0, &[mountain()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, ogre_shaman(), 0);
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

    assert!(on_battlefield(&engine, p1, gray_ogre()).is_none());
    assert!(in_graveyard(&engine, p0, mountain()).is_some());
    crate::engine::card_tests::legends_kit::float(&mut engine, p0, &[(ManaColor::Red, 2)]);
    let source = on_battlefield(&engine, p0, ogre_shaman()).unwrap();
    assert!(
        crate::engine::card_tests::partials_w1b_kit::offered(&engine, source).is_empty(),
        "empty hand cannot pay random discard"
    );
}
