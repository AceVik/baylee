//! `cards/creatures/mv_4/mistform_seaswift.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::subtypes;

fn mistform_seaswift() -> CardIndex {
    card_index("268f423e-b368-4561-ae24-db0f7ca775f4")
}

/// Mistform Seaswift can morph face down for `{3}`, turn face up for `{1}{U}`
/// as a 3/1 Flying Illusion, change its type for `{1}` until end of turn, and
/// revert at cleanup.
#[test]
fn mistform_seaswift_morphs_flies_and_changes_type_until_eot() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[island(), island(), island(), island(), island(), island()],
        )
        .hand(0, &[mistform_seaswift()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let first3: Vec<ObjectId> = lands_of(&engine, p0).into_iter().take(3).collect();
    tap_mana_where(&mut engine, p0, |id| first3.contains(&id));
    cast_with_floating(&mut engine, p0, mistform_seaswift());
    pass_until(&mut engine, stack_is_empty);

    let swift = on_battlefield(&engine, p0, mistform_seaswift()).expect("face-down permanent");
    assert!(
        engine
            .state()
            .object(swift)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN),
        "entered face down via morph"
    );
    assert_eq!(pt(&engine, swift), (2, 2));
    assert!(!keywords(&engine, swift).contains(KeywordSet::FLYING));

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: swift,
                ability_index: baylee_engine::choice::TURN_FACE_UP,
            },
        )
        .expect("turn face up for {1}{U}");
    assert_eq!(pt(&engine, swift), (3, 1));
    assert!(keywords(&engine, swift).contains(KeywordSet::FLYING));

    activate(&mut engine, p0, mistform_seaswift(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseSubtype(subtypes::creature::DRAKE))
        .expect("choose Drake");
    pass_until(&mut engine, |e| at_rest(e, p0));

    let subs = &engine
        .state()
        .object(swift)
        .unwrap()
        .characteristics()
        .subtypes;
    assert!(subs.contains(subtypes::creature::DRAKE));
    assert!(!subs.contains(subtypes::creature::ILLUSION));

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    assert!(
        engine
            .state()
            .object(swift)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(subtypes::creature::ILLUSION)
    );
}
