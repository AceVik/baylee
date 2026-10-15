//! `cards/creatures/artifacts/mv_4/shield_wall_sentinel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::choice::{ChoicePrompt, YesNoPrompt};

fn shield_wall_sentinel() -> CardIndex {
    card_index("4976f08c-6f8e-42e4-9584-f04793518ddb")
}

/// Amaranthine Wall, a creature card with defender.
fn amaranthine_wall() -> CardIndex {
    card_index("3c9fcda6-3ebc-4f9d-9f73-2bb4252d9420")
}

/// The Sentinel cast over a library of `filler`, stopped at "you may".
fn cast_sentinel_over(filler: CardIndex) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, filler)
        .battlefield(0, &[forest(); 4])
        .hand(0, &[shield_wall_sentinel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, shield_wall_sentinel());
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine
}

/// Shield-Wall Sentinel — {4} 1/3 with defender: "When this creature enters,
/// you may search your library for a creature card with defender, reveal it,
/// put it into your hand, then shuffle." Over a library of Amaranthine Walls,
/// saying yes puts one in hand.
#[test]
fn shield_wall_sentinel_fetches_a_defender_when_you_say_yes() {
    let p0 = PlayerId::new(0);
    let mut engine = cast_sentinel_over(amaranthine_wall());
    let library_before = library_size(&engine, p0);
    engine
        .apply(p0, PlayerAction::YesNo(true))
        .expect("the may is answered");
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
    assert!(!options.is_empty(), "the Walls have defender");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .expect("a Wall is on the menu");
    pass_until(&mut engine, stack_is_empty);

    let sentinel = on_battlefield(&engine, p0, shield_wall_sentinel()).expect("it resolved");
    assert_eq!(pt(&engine, sentinel), (1, 3));
    assert!(keywords(&engine, sentinel).contains(KeywordSet::DEFENDER));
    assert!(in_hand(&engine, p0, amaranthine_wall()).is_some());
    assert_eq!(library_size(&engine, p0), library_before - 1);
}

/// "May": saying no leaves the library and the hand alone.
#[test]
fn shield_wall_sentinel_fetches_nothing_when_you_say_no() {
    let p0 = PlayerId::new(0);
    let mut engine = cast_sentinel_over(amaranthine_wall());
    let library_before = library_size(&engine, p0);
    engine
        .apply(p0, PlayerAction::YesNo(false))
        .expect("the may is answered");
    pass_until(&mut engine, stack_is_empty);
    assert!(in_hand(&engine, p0, amaranthine_wall()).is_none());
    assert_eq!(library_size(&engine, p0), library_before);
}

/// A creature without defender is not a find: over Grizzly Bears, yes
/// fetches nothing.
#[test]
fn shield_wall_sentinel_finds_only_creatures_with_defender() {
    let p0 = PlayerId::new(0);
    let mut engine = cast_sentinel_over(grizzly_bears());
    let library_before = library_size(&engine, p0);
    engine
        .apply(p0, PlayerAction::YesNo(true))
        .expect("the may is answered");
    pass_until(&mut engine, stack_is_empty);
    assert!(in_hand(&engine, p0, grizzly_bears()).is_none());
    assert_eq!(library_size(&engine, p0), library_before);
}
