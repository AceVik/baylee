//! `cards/creatures/mv_8/razaketh_the_foulblooded.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn razaketh() -> CardIndex {
    card_index("136c9ecb-59b4-4ef6-bcb2-7b8d3df3ee75")
}

/// Razaketh is an 8/8 for {5}{B}{B}{B} with flying and trample.
#[test]
fn razaketh_is_cast_for_eight_with_flying_and_trample() {
    let mut engine = at_main(&[swamp(); 8], &[razaketh()]);
    let demon = cast_and_settle(&mut engine, razaketh());
    assert_eq!(pt(&engine, demon), (8, 8));
    assert!(keywords(&engine, demon).contains(KeywordSet::FLYING));
    assert!(keywords(&engine, demon).contains(KeywordSet::TRAMPLE));
}

/// "Pay 2 life, Sacrifice another creature: Search your library for a card,
/// put that card into your hand": the sacrifice menu lists the Bears and not
/// Razaketh, the Bears go to the graveyard, 2 life are paid and a card from
/// the library arrives in hand.
#[test]
fn razaketh_pays_life_and_a_creature_to_tutor() {
    let p0 = PlayerId::new(0);
    let mut engine = at_main(&[razaketh(), grizzly_bears()], &[]);
    let demon = on_battlefield(&engine, p0, razaketh()).expect("Razaketh");
    let bear = on_battlefield(&engine, p0, grizzly_bears()).expect("the bear");
    let (life, hand, library) = (
        life_of(&engine, 0),
        hand_size(&engine, 0),
        library_size(&engine, p0),
    );

    activate(&mut engine, p0, razaketh(), 0);
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("the sacrifice is chosen first: {:?}", engine.pending())
    };
    assert!(options.contains(&bear));
    assert!(!options.contains(&demon), "'another' creature: {options:?}");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bear],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options, prompt, ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!(
        prompt,
        ChoicePrompt::SearchLibrary,
        "then the library search"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, grizzly_bears()).is_some(),
        "sacrificed"
    );
    assert_eq!(life_of(&engine, 0), life - 2);
    assert_eq!(hand_size(&engine, 0), hand + 1, "the found card");
    assert_eq!(library_size(&engine, p0), library - 1);
}

/// Without another creature the ability cannot be paid for.
#[test]
fn razaketh_alone_cannot_activate() {
    let p0 = PlayerId::new(0);
    let engine = at_main(&[razaketh()], &[]);
    let demon = on_battlefield(&engine, p0, razaketh()).expect("Razaketh");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("priority expected")
    };
    assert!(!legal.abilities.iter().any(|(id, _)| *id == demon));
}
