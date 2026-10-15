//! `cards/creatures/artifacts/mv_4/plasmancer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::choice::ChoicePrompt;

fn plasmancer() -> CardIndex {
    card_index("b3ff8da4-f2cc-41c3-8950-9cc531854b78")
}

/// A seat 0 with two Swamps and two Forests, Plasmancer in hand, over a
/// library of `filler`.
fn cast_plasmancer_over(filler: CardIndex) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, filler)
        .battlefield(0, &[swamp(), swamp(), forest(), forest()])
        .hand(0, &[plasmancer()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, plasmancer());
    engine
}

/// Plasmancer — {2}{B}{B} 3/3 with flying: "Dynastic Advisor — When this
/// creature enters, search your library for a basic Swamp card, reveal it,
/// put it into your hand, then shuffle." Over a library of Swamps the search
/// offers them, one moves to the hand, and the library is one smaller.
#[test]
fn plasmancer_fetches_a_basic_swamp_to_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = cast_plasmancer_over(swamp());
    let library_before = library_size(&engine, p0);
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
        options, min, max, ..
    } = engine.pending().clone()
    else {
        unreachable!("the pass waited for this");
    };
    assert!(!options.is_empty(), "Swamps are in the library");
    assert_eq!((min, max), (0, 1), "up to one: a search may find nothing");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .expect("a Swamp is on the menu");
    pass_until(&mut engine, stack_is_empty);

    let witch = on_battlefield(&engine, p0, plasmancer()).expect("it resolved");
    assert_eq!(pt(&engine, witch), (3, 3));
    assert!(keywords(&engine, witch).contains(KeywordSet::FLYING));
    assert!(
        in_hand(&engine, p0, swamp()).is_some(),
        "the Swamp is in hand"
    );
    assert_eq!(library_size(&engine, p0), library_before - 1);
}

/// The same search over a library of Forests finds nothing: a basic land is
/// not enough, it has to be a Swamp.
#[test]
fn plasmancer_finds_no_swamp_among_forests() {
    let p0 = PlayerId::new(0);
    let mut engine = cast_plasmancer_over(forest());
    let library_before = library_size(&engine, p0);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, plasmancer()).is_some());
    assert!(in_hand(&engine, p0, forest()).is_none(), "nothing fetched");
    assert_eq!(library_size(&engine, p0), library_before);
}
