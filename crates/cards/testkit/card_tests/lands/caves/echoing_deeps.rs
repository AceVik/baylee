//! `cards/lands/caves/echoing_deeps.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn echoing_deeps_enters_as_copy_of_graveyard_land_with_cave_subtype() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[echoing_deeps()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    seed_graveyard(&mut engine, p0, 1);
    let graveyard_land = in_graveyard(&engine, p0, forest()).expect("forest in graveyard");

    let card = in_hand(&engine, p0, echoing_deeps()).expect("Echoing Deeps is in hand");
    engine
        .apply(p0, PlayerAction::PlayLand { card })
        .expect("land drop succeeds");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseTargets prompt for CopyOnEnter, got {:?}",
            engine.pending()
        );
    };
    assert!(
        options.contains(&graveyard_land),
        "graveyard forest is a legal target: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![graveyard_land],
                players: vec![],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    let deeps_obj =
        on_battlefield(&engine, p0, echoing_deeps()).expect("Echoing Deeps is on the battlefield");
    assert!(
        is_tapped(&engine, deeps_obj),
        "the chosen copy enters tapped"
    );

    let chars = engine
        .state()
        .object(deeps_obj)
        .expect("object exists")
        .characteristics();
    assert!(
        chars
            .subtypes
            .contains(baylee_core::generated::subtypes::land::CAVE),
        "retains the Cave subtype"
    );
    assert!(
        chars
            .subtypes
            .contains(baylee_core::generated::subtypes::land::FOREST),
        "copies the Forest subtype"
    );

    engine
        .dev_state_mut(p0)
        .unwrap()
        .set_tapped(deeps_obj, false);
    engine.refresh_offer();
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "copied Forest provides green mana"
    );
}

#[test]
fn echoing_deeps_declining_copy_enters_untapped() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[echoing_deeps()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    seed_graveyard(&mut e, p0, 1);
    let card = in_hand(&e, p0, echoing_deeps()).unwrap();
    e.apply(p0, PlayerAction::PlayLand { card }).unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(!is_tapped(
        &e,
        on_battlefield(&e, p0, echoing_deeps()).unwrap()
    ));
}
