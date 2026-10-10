//! `cards/artifacts/mv_2/thaumatic_compass.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The front fetches the seventh land and transforms at the end step. The back
/// untaps an opposing attacker and removes it from combat on the following turn.
#[test]
fn thaumatic_compass_searches_for_land_and_transforms_at_end_step_with_seven_lands() {
    use crate::engine::card_tests::legends_kit as kit;
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
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let compass = on_battlefield(&engine, p0, thaumatic_compass()).expect("compass on battlefield");
    let compass_was = identity(&engine, compass);
    assert_eq!(engine.state().object(compass).unwrap().face_index, 0);

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

    assert_eq!(lands_of(&engine, p0).len(), 7);

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
    reach_their_main_phase(&mut engine, p0);
    let attacker = kit::obj(&engine, kit::P1, llanowar_elves());
    pass_until(&mut engine, |e| {
        e.state().turn.active == kit::P1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            kit::P1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    kit::use_ability(&mut engine, p0, thaumatic_compass(), 1, &[attacker], &[]);
    assert!(!is_tapped(&engine, attacker));
    assert!(engine.state().combat.defender_of(attacker).is_none());
}
