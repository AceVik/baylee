//! `cards/creatures/artifacts/mv_4/scampering_surveyor.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::choice::ChoicePrompt;

fn scampering_surveyor() -> CardIndex {
    card_index("63c8f510-eba8-4f00-9f80-fbbc74801a4a")
}

/// A seat 0 with four Forests and a Scampering Surveyor in hand, over a
/// library of `filler`, the Surveyor cast.
fn cast_surveyor_over(filler: CardIndex) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, filler)
        .battlefield(0, &[forest(); 4])
        .hand(0, &[scampering_surveyor()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, scampering_surveyor());
    engine
}

/// Scampering Surveyor — {4} 3/2: "When this creature enters, search your
/// library for a basic land card or Cave card, put it onto the battlefield
/// tapped, then shuffle." A Forest from the library arrives tapped, beside
/// the four that cast the Surveyor.
#[test]
fn scampering_surveyor_puts_a_basic_land_onto_the_battlefield_tapped() {
    let p0 = PlayerId::new(0);
    let mut engine = cast_surveyor_over(forest());
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
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!("the pass waited for this");
    };
    assert!(!options.is_empty());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .expect("a basic land is on the menu");
    pass_until(&mut engine, stack_is_empty);

    let surveyor = on_battlefield(&engine, p0, scampering_surveyor()).expect("it resolved");
    assert_eq!(pt(&engine, surveyor), (3, 2));
    let forests = all_on_battlefield(&engine, p0, forest());
    assert_eq!(forests.len(), 5, "four plus the fetched one");
    // The four that paid for the Surveyor are tapped, so every Forest being
    // tapped is the fetched one having come in tapped.
    assert!(
        forests.iter().all(|id| is_tapped(&engine, *id)),
        "the fetched Forest came in tapped"
    );
    assert_eq!(library_size(&engine, p0), library_before - 1);
}

/// A library of Sol Rings holds no basic land and no Cave: nothing is
/// fetched, the board keeps its four lands.
#[test]
fn scampering_surveyor_finds_nothing_but_basic_lands_and_caves() {
    let p0 = PlayerId::new(0);
    let mut engine = cast_surveyor_over(sol_ring());
    let library_before = library_size(&engine, p0);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, scampering_surveyor()).is_some());
    assert_eq!(all_on_battlefield(&engine, p0, forest()).len(), 4);
    assert!(on_battlefield(&engine, p0, sol_ring()).is_none());
    assert_eq!(library_size(&engine, p0), library_before);
}
