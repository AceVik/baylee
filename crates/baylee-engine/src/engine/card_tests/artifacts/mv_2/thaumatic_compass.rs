//! `cards/artifacts/mv_2/thaumatic_compass.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Thaumatic Compass` // `Spires of Orazca` (`Coverage::Partial`): "{3}, {T}: Search your library
/// for a basic land card, reveal it, put it into your hand, then shuffle. At the beginning of
/// your end step, if you control seven or more lands, transform this artifact. // {T}: Add {C}.
/// {T}: Untap target attacking creature an opponent controls and remove it from combat."
///
/// Under `Coverage::Partial`, `Spires of Orazca`'s combat removal ability is omitted, while
/// the land search and the end-step transform under `Condition::ControlCount(&Filter::LAND, 7)`
/// are fully implemented. The test uses `Thaumatic Compass` to fetch a 7th basic land to hand,
/// plays that land, advances to the end step, and confirms that `Thaumatic Compass` transforms into
/// `Spires of Orazca` on face 1 as a land.
#[test]
fn thaumatic_compass_searches_for_land_and_transforms_at_end_step_with_seven_lands() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(249, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                thaumatic_compass(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let compass = on_battlefield(&engine, p0, thaumatic_compass()).expect("compass on battlefield");
    let compass_was = identity(&engine, compass);
    assert_eq!(
        engine.state().object(compass).map(|o| o.face_index),
        Some(0),
        "compass starts on face 0"
    );

    // Tap 3 Forests to activate the {3}, {T} search ability.
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, thaumatic_compass(), 0);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });

    let Pending::ChooseCards {
        options, prompt, ..
    } = engine.pending().clone()
    else {
        panic!("expected search prompt, got {:?}", engine.pending())
    };
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::SearchLibrary,
        "search library prompt"
    );
    assert!(
        !options.is_empty(),
        "library contains basic lands to search"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    let fetched_land = in_hand(&engine, p0, forest()).expect("searched land is in hand");
    engine
        .apply(p0, PlayerAction::PlayLand { card: fetched_land })
        .unwrap();

    assert_eq!(
        lands_of(&engine, p0).len(),
        7,
        "p0 now controls seven lands"
    );

    // Advance to the end step: with 7 lands, the transform trigger fires and resolves.
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, thaumatic_compass())
            .is_some_and(|id| e.state().object(id).map(|o| o.face_index) == Some(1))
    });

    let spires = on_battlefield(&engine, p0, thaumatic_compass()).expect("spires on battlefield");
    assert_eq!(
        identity(&engine, spires),
        compass_was,
        "the same object, turned over: a transform changes no zone (CR 712.18)"
    );
    assert!(
        is_tapped(&engine, spires),
        "still tapped from the search: turning over is not entering"
    );
    let t = types(&engine, spires);
    assert!(t.contains(TypeSet::LAND), "transformed permanent is a land");
    assert!(
        !t.contains(TypeSet::ARTIFACT),
        "transformed permanent is no longer an artifact"
    );
}
