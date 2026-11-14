use super::*;
use baylee_core::generated::index::{DARKSTEEL_CITADEL, GRIZZLY_BEARS};

fn fallow_earth() -> CardIndex {
    card_index("05a4acfd-3bf3-4ec2-a28a-bc43ba7c6fbf")
}

#[test]
fn fallow_earth_targets_only_lands_and_puts_on_top_of_owners_library() {
    let fe = fallow_earth();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[forest(), plains(), plains()])
        .battlefield(1, &[swamp(), DARKSTEEL_CITADEL, GRIZZLY_BEARS, sol_ring()])
        .hand(0, &[fe, fe])
        .start();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    reach_main_phase(&mut engine, p0);

    let swamp_id = on_battlefield(&engine, p1, swamp()).unwrap();
    let citadel_id = on_battlefield(&engine, p1, DARKSTEEL_CITADEL).unwrap();
    let bears_id = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    let ring_id = on_battlefield(&engine, p1, sol_ring()).unwrap();
    let p1_lib_before = library_size(&engine, p1);

    cast_from_hand(&mut engine, p0, fe);
    match engine.pending().clone() {
        Pending::ChooseTargets { options, .. } => {
            assert!(options.contains(&swamp_id) && options.contains(&citadel_id));
            assert!(!options.contains(&bears_id) && !options.contains(&ring_id));
        }
        other => panic!("expected ChooseTargets, got {other:?}"),
    }
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![bears_id],
                    players: vec![],
                },
            )
            .is_err(),
        "must not target nonland permanent"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![swamp_id],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(on_battlefield(&engine, p1, swamp()).is_none());
    assert!(in_graveyard(&engine, p1, swamp()).is_none());
    assert_eq!(library_size(&engine, p1), p1_lib_before + 1);
    let top = engine
        .state()
        .zones
        .list(ZoneLocation::Library(p1))
        .last()
        .copied();
    assert_eq!(top, Some(swamp_id));

    // Also works on an indestructible land (putting on library is not destroying).
    assert!(walk_to_own_main(&mut engine, p1));
    assert!(walk_to_own_main(&mut engine, p0));
    cast_from_hand(&mut engine, p0, fe);
    aim_at(&mut engine, p0, citadel_id);
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(on_battlefield(&engine, p1, DARKSTEEL_CITADEL).is_none());
    let new_top = engine
        .state()
        .zones
        .list(ZoneLocation::Library(p1))
        .last()
        .copied();
    assert_eq!(new_top, Some(citadel_id));
}

#[test]
fn fallow_earth_puts_own_land_on_own_library_top() {
    let fe = fallow_earth();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[forest(), mountain(), plains(), plains()])
        .hand(0, &[fe])
        .start();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    reach_main_phase(&mut engine, p0);

    let mtn_id = on_battlefield(&engine, p0, mountain()).unwrap();
    let lib_before = library_size(&engine, p0);

    cast_from_hand(&mut engine, p0, fe);
    aim_at(&mut engine, p0, mtn_id);
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(on_battlefield(&engine, p0, mountain()).is_none());
    assert_eq!(library_size(&engine, p0), lib_before + 1);
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(p0))
            .last()
            .copied(),
        Some(mtn_id)
    );
}
