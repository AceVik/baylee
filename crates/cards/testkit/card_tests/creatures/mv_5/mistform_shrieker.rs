//! `cards/creatures/mv_5/mistform_shrieker.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::subtypes;

fn mistform_shrieker() -> CardIndex {
    card_index("50fe31e9-473e-4d2f-b1b5-3eef3c7bb0e6")
}

/// Mistform Shrieker casts face down for `{3}` via morph, turns face up for
/// `{3}{U}{U}` as a 3/3 Flying Illusion, and changes creature type for `{1}`
/// until end of turn.
#[test]
fn mistform_shrieker_morphs_flies_and_changes_type_until_eot() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
            ],
        )
        .hand(0, &[mistform_shrieker()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let first3: Vec<ObjectId> = lands_of(&engine, p0).into_iter().take(3).collect();
    tap_mana_where(&mut engine, p0, |id| first3.contains(&id));
    cast_with_floating(&mut engine, p0, mistform_shrieker());
    pass_until(&mut engine, stack_is_empty);

    let shrieker = on_battlefield(&engine, p0, mistform_shrieker()).expect("face-down permanent");
    assert!(
        engine
            .state()
            .object(shrieker)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN),
        "entered face down via morph"
    );
    assert_eq!(pt(&engine, shrieker), (2, 2));
    assert!(!keywords(&engine, shrieker).contains(KeywordSet::FLYING));

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: shrieker,
                ability_index: baylee_engine::choice::TURN_FACE_UP,
            },
        )
        .expect("turn face up for {3}{U}{U}");
    assert_eq!(pt(&engine, shrieker), (3, 3));
    assert!(keywords(&engine, shrieker).contains(KeywordSet::FLYING));

    activate(&mut engine, p0, mistform_shrieker(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseSubtype(subtypes::creature::MERFOLK))
        .expect("choose Merfolk");
    pass_until(&mut engine, |e| at_rest(e, p0));

    let subs = &engine
        .state()
        .object(shrieker)
        .unwrap()
        .characteristics()
        .subtypes;
    assert!(subs.contains(subtypes::creature::MERFOLK));
    assert!(!subs.contains(subtypes::creature::ILLUSION));

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    assert!(
        engine
            .state()
            .object(shrieker)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(subtypes::creature::ILLUSION)
    );
}
