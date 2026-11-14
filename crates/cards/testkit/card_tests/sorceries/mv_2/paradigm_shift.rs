use super::*;

fn paradigm_shift() -> CardIndex {
    card_index("7179c2c5-c59c-4793-a3b5-28d26d755157")
}

#[test]
fn paradigm_shift_exiles_library_then_shuffles_graveyard_into_library() {
    let ps = paradigm_shift();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[island(), island()])
        .hand(0, &[ps, forest(), mountain(), swamp()])
        .hand(1, &[island()])
        .start();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    reach_main_phase(&mut engine, p0);

    let f = hand_to_graveyard(&mut engine, p0, forest());
    let m = hand_to_graveyard(&mut engine, p0, mountain());
    let s = hand_to_graveyard(&mut engine, p0, swamp());
    let _p1_gy = hand_to_graveyard(&mut engine, p1, island());

    let p0_lib_before = library_size(&engine, p0);
    let p1_lib_before = library_size(&engine, p1);

    cast_from_hand(&mut engine, p0, ps);
    pass_until(&mut engine, |e| at_rest(e, p0));

    // Step 1 exiled all cards that were in p0's library.
    assert_eq!(exiled(&engine, p0, plains()), p0_lib_before);
    // Step 2 shuffled p0's 3 graveyard cards into p0's library without exiling them.
    let p0_lib_after: Vec<ObjectId> = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    assert_eq!(p0_lib_after.len(), 3);
    assert!(p0_lib_after.contains(&f) && p0_lib_after.contains(&m) && p0_lib_after.contains(&s));

    // CR 608.2n: Paradigm Shift went to the graveyard after both steps finished.
    let gy_after = engine.state().zones.list(ZoneLocation::Graveyard(p0));
    assert_eq!(gy_after.len(), 1);
    assert!(in_graveyard(&engine, p0, ps).is_some());

    // Opponent's library and graveyard were not affected.
    assert_eq!(library_size(&engine, p1), p1_lib_before);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        1
    );
}

#[test]
fn paradigm_shift_with_empty_graveyard_leaves_library_empty() {
    let ps = paradigm_shift();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[island(), island()])
        .hand(0, &[ps])
        .start();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    reach_main_phase(&mut engine, p0);

    let p0_lib_before = library_size(&engine, p0);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        0
    );

    cast_from_hand(&mut engine, p0, ps);
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(exiled(&engine, p0, plains()), p0_lib_before);
    assert_eq!(library_size(&engine, p0), 0);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        1
    );
    assert!(in_graveyard(&engine, p0, ps).is_some());
}
