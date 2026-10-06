//! `cards/enchantments/mv_2/compulsion.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Compulsion` (`Coverage::Implemented`):
/// "`{{1}}{{U}}`, Discard a card: Draw a card. `{{1}}{{U}}`, Sacrifice this enchantment: Draw a card."
///
/// Verifies that activating `Compulsion`'s first ability costs `{{1}}{{U}}` and prompts
/// to discard a card from hand, subsequently drawing a card upon resolution.
#[test]
fn compulsion_discards_to_draw_a_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1329, forest())
        .battlefield(0, &[compulsion(), island(), island()])
        .hand(0, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let forest_card = in_hand(&engine, p0, forest()).expect("forest in hand");
    tap_all_mana(&mut engine, p0);

    activate(&mut engine, p0, compulsion(), 0);
    let Pending::ChooseCards {
        prompt, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected discard cost choice, got {:?}", engine.pending())
    };
    assert_eq!(prompt, ChoicePrompt::CostDiscard);
    assert!(options.contains(&forest_card));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![forest_card],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, forest()).is_some(),
        "discarded card is in graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, compulsion()).is_some(),
        "Compulsion remains on the battlefield"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        1,
        "drew a replacement card"
    );
}
