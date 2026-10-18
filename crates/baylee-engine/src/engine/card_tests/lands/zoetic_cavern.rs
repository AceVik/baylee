//! `cards/lands/zoetic_cavern.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Zoetic Cavern: "{T}: Add {C}." / "Morph {2}"
/// Activating ability 0 adds one colorless mana to the pool and leaves the land tapped.
#[test]
fn zoetic_cavern_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(138, forest())
        .battlefield(0, &[zoetic_cavern()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let cavern = on_battlefield(&engine, p0, zoetic_cavern()).expect("Zoetic Cavern deployed");
    activate(&mut engine, p0, zoetic_cavern(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, cavern));
}

#[test]
fn zoetic_cavern_casts_face_down_with_morph_and_turns_face_up_for_two_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(139, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .hand(0, &[zoetic_cavern()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, zoetic_cavern());
    pass_until(&mut engine, stack_is_empty);

    let cavern = on_battlefield(&engine, p0, zoetic_cavern()).expect("Zoetic Cavern resolved");
    assert!(
        engine
            .state()
            .object(cavern)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN),
        "entered face down via morph"
    );
    assert!(types(&engine, cavern).contains(TypeSet::CREATURE));
    assert!(!types(&engine, cavern).contains(TypeSet::LAND));
    assert_eq!(pt(&engine, cavern), (2, 2));

    // Turn face up for {2} via the special action TURN_FACE_UP.
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: cavern,
                ability_index: crate::choice::TURN_FACE_UP,
            },
        )
        .expect("turn Zoetic Cavern face up for {2}");

    assert!(
        !engine
            .state()
            .object(cavern)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN),
        "Zoetic Cavern is now face up"
    );
    assert!(types(&engine, cavern).contains(TypeSet::LAND));
    assert!(!types(&engine, cavern).contains(TypeSet::CREATURE));

    activate(&mut engine, p0, zoetic_cavern(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1
    );
}
