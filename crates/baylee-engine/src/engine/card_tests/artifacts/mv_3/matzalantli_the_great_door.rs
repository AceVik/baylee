//! `cards/artifacts/mv_3/matzalantli_the_great_door.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Matzalantli, the Great Door` // `The Core` (`Coverage::Partial`):
/// "`{{T}}`: Draw a card, then discard a card. `{{4}}`, `{{T}}`: Transform `Matzalantli`.
/// Activate only if there are four or more permanent types among cards in your graveyard.
/// // Fathomless descent — `{{T}}`: Add X mana of any one color, where X is the number
/// of permanent cards in your graveyard."
///
/// Under `Coverage::Partial`, the `{{4}}`, `{{T}}` transform ability is omitted because no
/// condition counts permanent types in a graveyard, leaving the front-face `{{T}}` loot ability.
/// The test verifies that with four mana floating and `Matzalantli` untapped, `LegalActions::abilities`
/// offers only ability 0 and no transform ability. It then activates ability 0, answers the
/// `ChoicePrompt::Generic` card-choice prompt to discard, and confirms the drawn card and graveyard entry.
#[test]
fn matzalantli_the_great_door_draws_and_discards_and_omits_transform() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(302, forest())
        .battlefield(
            0,
            &[
                matzalantli_the_great_door(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let door = on_battlefield(&engine, p0, matzalantli_the_great_door())
        .expect("Matzalantli on battlefield");
    assert!(!is_tapped(&engine, door), "Matzalantli starts untapped");

    // Float four mana to verify that no {4}, {T} transform ability is offered.
    tap_mana_except(&mut engine, p0, door);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four mana floating in pool"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    let door_abilities: Vec<u32> = legal
        .abilities
        .iter()
        .filter(|(src, _)| *src == door)
        .map(|(_, idx)| *idx)
        .collect();
    assert_eq!(
        door_abilities,
        vec![0],
        "under `Coverage::Partial` only ability 0 is offered; the {{4}}, {{T}} transform is omitted"
    );

    // Ability 0: "{T}: Draw a card, then discard a card."
    activate(&mut engine, p0, matzalantli_the_great_door(), 0);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on card choice");
    };
    assert_eq!(player, p0, "p0 must choose a card to discard");
    assert_eq!((min, max), (1, 1), "must discard exactly one card");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::Generic,
        "prompt is ChoicePrompt::Generic"
    );
    assert_eq!(
        options.len(),
        2,
        "hand has the initial card plus the drawn card"
    );

    let chosen = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, door), "Matzalantli tapped to pay cost");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        1,
        "hand size returned to 1 after drawing and discarding"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        1,
        "discarded card is in the graveyard"
    );
}
