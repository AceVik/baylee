//! `cards/creatures/artifacts/mv_2/sundial_dawn_tyrant.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn sundial_dawn_tyrant() -> CardIndex {
    card_index("9ca808c5-9338-4199-a45b-df1a1a20625e")
}

/// Sundial, Dawn Tyrant prints no rules text: a `{1}{W}` 3/3 legendary
/// artifact creature, castable only with white mana in the pool.
#[test]
fn sundial_dawn_tyrant_is_a_three_three_for_one_and_white() {
    let (engine, sundial) = vanilla_cast(sundial_dawn_tyrant(), &[plains(), forest()], (3, 3));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert!(!is_tapped(&engine, sundial));
}

/// Without white mana the `{W}` cannot be paid: the spell is not on offer.
#[test]
fn sundial_dawn_tyrant_cannot_be_cast_without_white() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[sundial_dawn_tyrant()], &[forest(), forest()], &[]);
    tap_all_mana(&mut engine, p0);
    let sundial = in_hand(&engine, p0, sundial_dawn_tyrant()).expect("in hand");
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: sundial })
            .is_err(),
        "two green mana do not pay {{1}}{{W}}"
    );
}

/// Legendary: a second Sundial under the same controller leaves one behind.
#[test]
fn sundial_dawn_tyrant_is_subject_to_the_legend_rule() {
    let p0 = PlayerId::new(0);
    let mut engine = table(
        &[sundial_dawn_tyrant()],
        &[sundial_dawn_tyrant(), plains(), forest()],
        &[],
    );
    let first = on_battlefield(&engine, p0, sundial_dawn_tyrant()).expect("seated");
    cast_from_hand(&mut engine, p0, sundial_dawn_tyrant());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::LegendChoice { .. })
    });
    let Pending::LegendChoice { player, options } = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(player, p0);
    assert_eq!(options.len(), 2, "both Sundials are offered");
    assert!(options.contains(&first));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![first],
            },
        )
        .expect("keep the first");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        all_on_battlefield(&engine, p0, sundial_dawn_tyrant()),
        vec![first],
        "the one kept stays"
    );
    assert_eq!(
        graveyard_count(&engine, 0),
        1,
        "the other went to the graveyard"
    );
}
