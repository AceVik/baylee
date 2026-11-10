//! `cards/creatures/mv_4/seismic_mage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Seismic Mage` is a four-mana 1/1 Human Spellshaper under `Coverage::Implemented`.
/// Its activated ability costs `{2}{R}`, tapping, and discarding a card to destroy target land.
/// Following the activation sequence (CR 601.2c before CR 601.2h), targeting the opponent's land is chosen
/// before discarding a card from hand as the cost prompted by `ChoicePrompt::CostDiscard`,
/// successfully tapping the creature, spending the floating mana, and destroying the targeted land upon resolution.
#[test]
fn seismic_mage_activates_to_destroy_target_land() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[seismic_mage(), mountain(), mountain(), mountain()])
        .battlefield(1, &[badlands()])
        .hand(0, &[mountain()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mage = on_battlefield(&engine, p0, seismic_mage()).expect("Seismic Mage is on battlefield");
    let opponent_land =
        on_battlefield(&engine, p1, badlands()).expect("Badlands is on battlefield");
    let fodder = in_hand(&engine, p0, mountain()).expect("card to discard is in hand");

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, seismic_mage(), 0);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseTargets for Seismic Mage, got {:?}",
            engine.pending()
        );
    };
    assert!(
        options.contains(&opponent_land),
        "opponent's land is a legal target"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![opponent_land],
                players: vec![],
            },
        )
        .unwrap();

    let Pending::ChooseCards {
        prompt,
        options: discard_options,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected ChooseCards for discard cost, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(
        prompt,
        ChoicePrompt::CostDiscard,
        "discard prompt is CostDiscard"
    );
    assert!(
        discard_options.contains(&fodder),
        "hand card is offered to pay discard cost"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .unwrap();

    assert!(
        is_tapped(&engine, mage),
        "Seismic Mage is tapped after activation"
    );
    assert!(
        in_graveyard(&engine, p0, mountain()).is_some(),
        "discarded card is in graveyard"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, badlands()).is_some(),
        "targeted land was destroyed and is now in graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, badlands()).is_none(),
        "targeted land is no longer on battlefield"
    );
}
