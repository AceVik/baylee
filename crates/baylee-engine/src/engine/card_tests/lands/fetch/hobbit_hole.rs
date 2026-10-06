//! `cards/lands/fetch/hobbit_hole.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Hobbit Hole prints `{{T}}, Sacrifice this land: Search your library for a basic land card, put it onto the battlefield tapped, then shuffle` and `Halflingcycling {{4}}`.
/// The card is marked `Coverage::Implemented`.
/// When played onto the battlefield, activating its fetch ability sacrifices the land and puts a searched basic land onto the battlefield tapped.
#[test]
fn hobbit_hole_sacrifices_on_battlefield_to_fetch_basic_land_tapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest()).hand(0, &[hobbit_hole()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hole = play_land(&mut engine, p0, hobbit_hole());
    assert!(!is_tapped(&engine, hole));

    activate(&mut engine, p0, hobbit_hole(), 0);
    assert!(in_graveyard(&engine, p0, hobbit_hole()).is_some());

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

    assert_eq!(
        engine.state().object(chosen).unwrap().zone,
        Zone::Battlefield
    );
    assert!(is_tapped(&engine, chosen));
}
