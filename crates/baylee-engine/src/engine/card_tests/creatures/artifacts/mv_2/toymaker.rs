//! `cards/creatures/artifacts/mv_2/toymaker.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

// oracle_id = "eb80f233-76b5-4c72-8a4a-f22e56314690"
fn toymaker() -> CardIndex {
    card_index("eb80f233-76b5-4c72-8a4a-f22e56314690")
}

/// `Toymaker` (`Coverage::Implemented`):
/// "{1}, {T}, Discard a card: Target noncreature artifact becomes an artifact
/// creature with power and toughness each equal to its mana value until end of
/// turn."
#[test]
fn toymaker_animates_target_noncreature_artifact_with_pt_equal_to_mana_value() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(9701, forest())
        .battlefield(0, &[toymaker(), index::JAYEMDAE_TOME, forest()])
        .hand(0, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let maker = on_battlefield(&engine, p0, toymaker()).expect("Toymaker");
    let tome = on_battlefield(&engine, p0, index::JAYEMDAE_TOME).expect("Jayemdae Tome");
    let discard_card = in_hand(&engine, p0, forest()).expect("card to discard");

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: maker,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![tome],
                players: vec![],
            },
        )
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![discard_card],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(
        types(&engine, tome).contains(TypeSet::CREATURE),
        "Jayemdae Tome becomes an artifact creature until end of turn"
    );
    assert_eq!(
        pt(&engine, tome),
        (4, 4),
        "Jayemdae Tome ({{4}}) has power and toughness 4/4 until end of turn"
    );
}
