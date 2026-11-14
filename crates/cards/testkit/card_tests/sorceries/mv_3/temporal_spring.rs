use super::*;
use baylee_core::generated::index::{CONTROL_MAGIC, GRIZZLY_BEARS, HILL_GIANT};

fn temporal_spring() -> CardIndex {
    card_index("1d7a4e9c-b3c9-421e-b5f4-82d51d7d5125")
}

#[test]
fn temporal_spring_puts_any_permanent_type_on_owners_library_top() {
    let ts = temporal_spring();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[forest(), island(), plains(), forest(), island(), plains()],
        )
        .battlefield(1, &[GRIZZLY_BEARS, sol_ring(), swamp()])
        .hand(0, &[ts, ts])
        .start();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    let ring = on_battlefield(&engine, p1, sol_ring()).unwrap();
    let p1_swamp = on_battlefield(&engine, p1, swamp()).unwrap();
    let lib_before = library_size(&engine, p1);

    cast_from_hand(&mut engine, p0, ts);
    match engine.pending().clone() {
        Pending::ChooseTargets { options, .. } => {
            assert!(
                options.contains(&bears) && options.contains(&ring) && options.contains(&p1_swamp)
            );
        }
        other => panic!("expected ChooseTargets, got {other:?}"),
    }
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(on_battlefield(&engine, p1, GRIZZLY_BEARS).is_none());
    assert_eq!(library_size(&engine, p1), lib_before + 1);
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(p1))
            .last()
            .copied(),
        Some(bears)
    );

    // Noncreature permanent (artifact) also works.
    cast_from_hand(&mut engine, p0, ts);
    aim_at(&mut engine, p0, ring);
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(on_battlefield(&engine, p1, sol_ring()).is_none());
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(p1))
            .last()
            .copied(),
        Some(ring)
    );
}

#[test]
fn temporal_spring_puts_stolen_permanent_on_its_owners_library_top() {
    let ts = temporal_spring();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                island(),
                island(),
                forest(),
                forest(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .battlefield(1, &[HILL_GIANT])
        .hand(0, &[CONTROL_MAGIC, ts])
        .start();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    reach_main_phase(&mut engine, p0);

    let giant = on_battlefield(&engine, p1, HILL_GIANT).unwrap();
    cast_from_hand(&mut engine, p0, CONTROL_MAGIC);
    aim_at(&mut engine, p0, giant);
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(on_battlefield(&engine, p0, HILL_GIANT), Some(giant));

    assert!(walk_to_own_main(&mut engine, p1));
    assert!(walk_to_own_main(&mut engine, p0));

    let p0_lib_before = library_size(&engine, p0);
    let p1_lib_before = library_size(&engine, p1);
    cast_from_hand(&mut engine, p0, ts);
    aim_at(&mut engine, p0, giant);
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(library_size(&engine, p0), p0_lib_before);
    assert_eq!(library_size(&engine, p1), p1_lib_before + 1);
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(p1))
            .last()
            .copied(),
        Some(giant),
        "stolen permanent goes to owner's library, not controller's"
    );
    assert!(in_graveyard(&engine, p0, CONTROL_MAGIC).is_some());
}
