use super::*;

fn reclaim() -> CardIndex {
    card_index("797155cd-faf4-4321-8629-c8c352392748")
}

#[test]
fn reclaim_puts_target_card_from_graveyard_on_top_of_library() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[reclaim()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    seed_graveyard(&mut engine, p0, 1);

    let lib_before = engine.state().zones.list(ZoneLocation::Library(p0)).len();
    let card_in_gy = in_graveyard(&engine, p0, forest()).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, reclaim());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![card_in_gy],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, forest()).is_none());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(p0)).last(),
        Some(&card_in_gy)
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(p0)).len(),
        lib_before + 1
    );
}
