//! `cards/lands/caves/echoing_deeps.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Echoing Deeps` is a land with subtype Cave under `Coverage::Partial`.
/// It prints "You may have this land enter tapped as a copy of any land card in a graveyard, except it's a Cave in addition to its other types."
/// Under `Coverage::Partial`, it enters untapped as a copy of a graveyard land card, retaining the Cave subtype.
/// When played while a `forest()` sits in the graveyard, it copies the Forest, gains the Cave subtype, and produces green mana.
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
        !is_tapped(&engine, deeps_obj),
        "under `Coverage::Partial` it arrives untapped"
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

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "copied Forest provides green mana"
    );
}
