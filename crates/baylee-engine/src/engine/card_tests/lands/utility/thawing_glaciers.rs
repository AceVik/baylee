//! `cards/lands/utility/thawing_glaciers.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Thawing Glaciers` prints `This land enters tapped.` and `{{1}}, {{T}}: Search your library for a basic land card, put that card onto the battlefield tapped, then shuffle. Return this land to its owner's hand at the beginning of the next cleanup step.`
///
/// Under `Coverage::Partial`, `EnterModifier::Tapped` and the search ability putting a basic land onto the battlefield tapped are implemented, while the delayed return to hand trigger is omitted.
/// Playing this land puts it onto the battlefield tapped; after untapping on the next turn, floating `{{1}}` allows activating ability 0 to search the library via `ChoicePrompt::SearchLibrary` and put a basic land onto the battlefield tapped.
#[test]
fn thawing_glaciers_enters_tapped_and_fetches_basic_land_tapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[thawing_glaciers()])
        .battlefield(0, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let card = in_hand(&engine, p0, thawing_glaciers()).expect("thawing glaciers in hand");
    engine.apply(p0, PlayerAction::PlayLand { card }).unwrap();

    let glaciers =
        on_battlefield(&engine, p0, thawing_glaciers()).expect("glaciers on battlefield");
    assert!(entered_tapped(&engine, glaciers));

    // Advance to p0's next main phase to untap.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, glaciers));

    // Float {{1}} from Forest while keeping Thawing Glaciers untapped.
    tap_mana_except(&mut engine, p0, glaciers);
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);
    assert!(!is_tapped(&engine, glaciers));

    activate(&mut engine, p0, thawing_glaciers(), 0);

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
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected SearchLibrary prompt, got {:?}", engine.pending());
    };
    assert_eq!(prompt, ChoicePrompt::SearchLibrary);
    assert_eq!((min, max), (1, 1));
    assert!(!options.is_empty(), "library contains basic land filler");

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
        engine.state().object(chosen).expect("chosen exists").zone,
        Zone::Battlefield
    );
    assert!(
        is_tapped(&engine, chosen),
        "the fetched basic land enters tapped"
    );
    assert!(is_tapped(&engine, glaciers));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}
