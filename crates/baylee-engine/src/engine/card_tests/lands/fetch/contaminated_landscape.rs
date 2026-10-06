//! `cards/lands/fetch/contaminated_landscape.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Contaminated Landscape` taps for `{{C}}`, cycles for `{{W}}{{U}}{{B}}`, and fetches a basic
/// Plains, Island, or Swamp tapped via `{{T}}`, sacrifice under `Coverage::Implemented`.
/// Activating the fetch ability pays its tap and sacrifice cost, searches the library, and puts
/// the selected basic land onto the battlefield tapped.
#[test]
fn contaminated_landscape_fetches_tapped_basic_island() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1906, island())
        .hand(0, &[contaminated_landscape()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, contaminated_landscape());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, contaminated_landscape(), 2);
    assert!(on_battlefield(&engine, p0, contaminated_landscape()).is_none());
    assert!(in_graveyard(&engine, p0, contaminated_landscape()).is_some());

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::SearchLibrary,
                ..
            }
        )
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
        panic!("expected search prompt, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (1, 1));
    assert_eq!(prompt, ChoicePrompt::SearchLibrary);
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

    assert!(entered_tapped(&engine, chosen));
    assert_eq!(lands_of(&engine, p0).len(), 1);
}
