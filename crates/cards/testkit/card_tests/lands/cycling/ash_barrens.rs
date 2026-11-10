//! `cards/lands/cycling/ash_barrens.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ash Barrens prints `{T}: Add {C}` and `Basic landcycling {1}`.
/// The card is marked `Coverage::Implemented`.
/// Activating basic landcycling from hand for `{1}` discards Ash Barrens into the graveyard
/// and searches the library for a basic land card, placing the chosen card into the player's hand.
#[test]
fn ash_barrens_cycles_from_hand_to_find_a_basic_land() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[ash_barrens()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    assert!(in_hand(&engine, p0, ash_barrens()).is_some());
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1
    );

    activate(&mut engine, p0, ash_barrens(), 1);
    assert!(in_graveyard(&engine, p0, ash_barrens()).is_some());

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::SearchLibrary,
                ..
            }
        )
    });

    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected library search prompt");
    };
    assert!(!options.is_empty());
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
    assert_eq!(engine.state().object(chosen).unwrap().zone, Zone::Hand);
}
