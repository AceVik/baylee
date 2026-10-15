//! `cards/creatures/artifacts/mv_1/gene_pollinator.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn gene_pollinator() -> CardIndex {
    card_index("0a356d04-2e86-435b-9766-27b93b468a61")
}

/// "{T}, Tap an untapped permanent you control: Add one mana of any color."
/// The price is a second permanent: it asks which one, then which color, and
/// both the Pollinator and the chosen Forest end tapped with one mana of the
/// chosen color (not green) floating.
#[test]
fn gene_pollinator_taps_another_permanent_for_one_mana_of_any_color() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[gene_pollinator(), forest()], &[]);
    let me = on_battlefield(&engine, p0, gene_pollinator()).expect("seated");
    let land = on_battlefield(&engine, p0, forest()).expect("forest");
    assert_eq!(pt(&engine, me), (1, 2));

    activate(&mut engine, p0, gene_pollinator(), 0);
    let Pending::ChooseCards {
        options, prompt, ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected the tap cost's question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(prompt, ChoicePrompt::CostTap);
    assert_eq!(options, vec![land], "the Pollinator itself is not on offer");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![land],
            },
        )
        .expect("the Forest was offered");
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("any color");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { .. })
    });
    assert!(is_tapped(&engine, me));
    assert!(is_tapped(&engine, land));
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Blue), 1);
}

/// With nothing else untapped there is nothing to pay the second tap with:
/// the Pollinator alone is not offered.
#[test]
fn gene_pollinator_alone_cannot_pay_its_own_cost() {
    let p0 = PlayerId::new(0);
    let engine = main_with(&[gene_pollinator()], &[]);
    let me = on_battlefield(&engine, p0, gene_pollinator()).expect("seated");
    assert!(!is_offered(&engine, me, 0));
}
