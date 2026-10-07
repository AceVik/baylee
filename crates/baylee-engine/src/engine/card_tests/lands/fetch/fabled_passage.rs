//! `cards/lands/fetch/fabled_passage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fabled Passage prints `{T}, Sacrifice this land: Search your library for a basic land card, put it
/// onto the battlefield tapped, then shuffle. Then if you control four or more lands, untap that land.`
/// The card is marked `Coverage::Partial` because untapping the fetched land conditionally is unsupported.
/// Activating Fabled Passage sacrifices it to the graveyard, searches a basic `forest` onto the battlefield
/// tapped, and leaves it tapped even when four or more lands are controlled.
#[test]
fn fabled_passage_sacrifices_to_fetch_a_basic_land_tapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[fabled_passage()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let passage = play_land(&mut engine, p0, fabled_passage());
    assert!(!is_tapped(&engine, passage));

    activate(&mut engine, p0, fabled_passage(), 0);
    assert!(in_graveyard(&engine, p0, fabled_passage()).is_some());

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
        panic!("expected library search");
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
