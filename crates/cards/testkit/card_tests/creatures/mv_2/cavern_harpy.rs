//! `cards/creatures/mv_2/cavern_harpy.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Cavern Harpy — {U}{B} Creature — Harpy Beast 2/1
/// "Flying
/// When this creature enters, return a blue or black creature you control to its owner's hand.
/// Pay 1 life: Return this creature to its owner's hand."
#[test]
fn cavern_harpy_returns_blue_or_black_creature_on_etb_and_bounces_itself_for_one_life() {
    let harpy = card_index("81c40bd8-b989-41e8-9527-eae8fb87311d");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                island(),
                swamp(),
                index::FUGITIVE_WIZARD,
                index::GRIZZLY_BEARS,
            ],
        )
        .hand(0, &[harpy])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wizard_id = on_battlefield(&engine, p0, index::FUGITIVE_WIZARD).unwrap();
    let bears_id = on_battlefield(&engine, p0, index::GRIZZLY_BEARS).unwrap();

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, harpy);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCards for Cavern Harpy ETB trigger");
    };
    assert!(
        options.contains(&wizard_id),
        "blue creature you control is eligible"
    );
    assert!(
        !options.contains(&bears_id),
        "green creature is not eligible for Cavern Harpy's blue-or-black bounce"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wizard_id],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, index::FUGITIVE_WIZARD).is_none());
    assert!(in_hand(&engine, p0, index::FUGITIVE_WIZARD).is_some());
    let harpy_id = on_battlefield(&engine, p0, harpy).expect("Cavern Harpy remains on battlefield");
    assert!(keywords(&engine, harpy_id).contains(KeywordSet::FLYING));

    // Activated ability (index 1, after triggered ability 0): Pay 1 life: Return Cavern Harpy to its owner's hand.
    assert_eq!(engine.state().players[0].life, 20);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: harpy_id,
                ability_index: 1,
            },
        )
        .unwrap();
    assert_eq!(engine.state().players[0].life, 19);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, harpy).is_none());
    assert!(in_hand(&engine, p0, harpy).is_some());
}
