//! `cards/instants/mv_2/karn_s_touch.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "b5f9920d-7af5-4876-8823-6efbf2b4d9b9"
fn karn_s_touch() -> CardIndex {
    card_index("b5f9920d-7af5-4876-8823-6efbf2b4d9b9")
}

/// `Karn's Touch` (`Coverage::Generated`):
/// "Target noncreature artifact becomes an artifact creature with power and toughness
/// each equal to its mana value until end of turn."
///
/// Verifies that `Karn's Touch` animates a mana-value-4 noncreature artifact (`Sisay's Ring`)
/// into a 4/4 artifact creature until end of turn, and that it reverts at cleanup.
#[test]
fn karns_touch_animates_noncreature_artifact_to_its_mana_value_until_eot() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(9415, island())
        .battlefield(0, &[island(), island(), sisay_s_ring()])
        .hand(0, &[karn_s_touch()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let ring = on_battlefield(&engine, p0, sisay_s_ring()).expect("Sisay's Ring on battlefield");
    cast_from_hand(&mut engine, p0, karn_s_touch());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ring],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .object(ring)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::CREATURE),
        "Sisay's Ring becomes an artifact creature"
    );
    assert_eq!(
        pt(&engine, ring),
        (4, 4),
        "power and toughness equal its mana value (4)"
    );

    // At end of turn, it reverts to a noncreature artifact.
    assert!(walk_to_own_main(&mut engine, p1));
    assert!(
        !engine
            .state()
            .object(ring)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::CREATURE),
        "Sisay's Ring is no longer a creature after end of turn"
    );
}
